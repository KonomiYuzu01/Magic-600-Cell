"""Two-level P*: the declared procedure, and at each of its stops family R without a beam.

A segment is a run of P* (`pstar.py run`) with the declared options. When a
segment stops, family R is searched again at the stop with the same depth and
no beam: by default with `family_r_full` (`--stop-search pruned`), or with
`pstar.step` and a beam of `--stop-beam` (`--stop-search beam`), which is
exhaustive when the beam is at least as large as every frontier; the record
keeps each target's truncation flag. The best candidate found there becomes the
first macro of the next segment, which certifies it again before committing it.
P* is memoryless, so a run that ends on its CPU limit is continued by a new run
from its last configuration. The CPU limit covers the whole run; outside its
guards run only the start of a segment (`pstar.run_procedure` loads its prefix
and checkpoints, as in `pstar.py run`) and this driver's summary writes.
`--stop-only` runs one stop search at the configuration of `--prefix`.
Run from the repository root; all outputs must be outside the repository.
"""
import argparse
import json
from pathlib import Path
import sys
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import pstar as S  # noqa: E402

A, P, J, CLS, np = S.A, S.P, S.J, S.CLS, S.np


def flat(prefix, words):
    return [tuple(p) for p in prefix] + [tuple(p) for word in words for p in word]


def family_r_full(state, depth, deadline=None):
    """Family R of `pstar.family_r` without a beam, depth first, with two exact reductions.

    The helper grips at each node (`RetainedSearch.generator`), the letters and
    the leaves are those of `pstar.family_r` with a beam larger than every
    frontier, and every target gets the same best candidate:
    - Two consecutive letters at one grip compose to one retained item there
      (Lemma RA), and that grip is a helper where the first of them acts, so the
      word without the repetition is in the search, reaches the same
      configuration and has a smaller key (it is shorter). Such words and their
      extensions are skipped.
    - A last letter at c moves only the pieces inside c: straddlers of the
      target outside c stay straddlers, its foreign pieces outside c stay
      foreign, and D gains at most the high pieces inside c. A last letter at c
      is tried only when every straddler of the target lies inside c and the
      foreign load outside c is below the largest D outside c plus the weight of
      the high pieces inside c; otherwise no lowering after it descends.
    """
    started = time.process_time()
    rs = S.RetainedSearch(state)
    phi0 = J.phi(state.h)
    targets = sorted({x for _, _, holders in P.high_units(state) for x in holders})
    counts = dict(targets=len(targets), nodes=0, retained_letters=0, lowering_tests=0, descending_words=0,
                  skipped_repeats=0, pruned_letters=0)
    best = {}
    per_target = []
    for x in targets:
        S.tick(deadline)
        by_depth = [0] * (depth + 1)
        codes = S.seconds().codes(x)

        def visit(prefix, metric):
            s, f, ds = metric
            if s:
                return
            counts['lowering_tests'] += 48
            for i, d in zip(P.JUMBLES, ds.tolist()):
                if f < d:
                    counts['descending_words'] += 1
                    S.keep_best(best, x, S.raw_candidate([*prefix, (x, i)], phi0 + 2*(f-d), 0))

        def may_descend(base, moved):
            s, f, ds = base
            w, lx = rs.w[moved], rs.lab[moved, x]
            if s != int(w[lx == 2].sum()):
                return False
            inside = lx == 1
            outside_f = f - int(w[inside & (rs.b[moved] != CLS[x])].sum())
            high = rs.sec[moved] >= 0
            outside_d = ds
            if (high & inside).any():
                dc = np.zeros(20, np.int64)
                np.add.at(dc, rs.sec[moved][high & inside], w[high & inside])
                outside_d = ds - dc[codes]
            return outside_f < int(outside_d.max()) + int(w[high].sum())

        def expand(prefix):
            S.tick(deadline)
            counts['nodes'] += 1
            base = rs.metric(x)
            last = len(prefix) + 1 == depth
            for c in rs.generator(x):
                if prefix and prefix[-1][0] == c:
                    counts['skipped_repeats'] += 11
                    continue
                moved = np.flatnonzero(rs.lab[:, c] == 1)
                if last and not may_descend(base, moved):
                    counts['pruned_letters'] += 11
                    continue
                for i in range(1, 12):
                    counts['retained_letters'] += 1
                    events = (*prefix, (c, i))
                    by_depth[len(events)] += 1
                    visit(events, rs.leaf(x, c, i, base, moved))
                    if not last:
                        with rs.applied(c, i):
                            expand(events)

        visit((), rs.metric(x))
        if depth >= 1:
            expand(())
        per_target.append(dict(target=x, nodes_at_depth=by_depth, beam_truncated=False))
    rows = S.replay_candidates(state, best.values(), 0, deadline)
    return dict(candidates=rows, counts=counts, exhausted=True, exhaustive_through=depth, depth=depth, beam=None,
                search='R generator of pstar.family_r without a beam; repeated grips skipped; last letters pruned '
                       'by an exact necessary condition',
                reporting='minimum choice key per target; descending words counted among the leaves tried',
                targets=per_target, cpu_s=round(time.process_time()-started, 6))


def stop_search(state, mode, depth, e0, stop_beam, deadline=None):
    if mode == 'beam':
        return S.step(state, S.options('R', e0, depth, stop_beam, 'best'), deadline)
    start, digest = time.process_time(), state.st.digest()
    record = family_r_full(state, depth, deadline)
    A.require(state.st.digest() == digest, 'undo', 'family R without a beam')
    candidates = [dict(c, family='R') for c in record['candidates']]
    return dict(format='pstar2-stop/1', digest=digest, phi=J.phi(state.h),
                options=dict(families=['R'], e0=e0, depth=depth, search='pruned'), families=dict(R=record),
                choice=min(candidates, key=S.candidate_key) if candidates else None,
                cpu_s=round(time.process_time()-start, 6))


def run(table, out, prefix, opts, stop_opts, labels=None, first=None, cpu_limit=None, max_segments=100):
    started = time.process_time()
    out = P.external_path(out)
    A.require(not (out/'pstar2.json').exists(), 'run exists', 'use a fresh output directory')
    A.require(cpu_limit is None or cpu_limit > 0, 'cpu-limit')
    words = []
    summary = dict(format='pstar2/2', table_digest=S.BM.load_table(table).digest(),
                   prefix=[list(p) for p in prefix], options=opts, stop_options=stop_opts,
                   first=[list(p) for p in first] if first is not None else None,
                   segments=[], macro_words=words, status='unknown', reason='initialised')

    def save():
        summary['cpu_s'] = round(time.process_time() - started, 6)
        S.atomic_json(out/'pstar2.json', summary)

    def remaining():
        return None if cpu_limit is None else cpu_limit - (time.process_time() - started)

    for k in range(max_segments):
        budget = remaining()
        if budget is not None and budget <= 0:
            summary.update(status='unknown', reason='cpu-limit')
            break
        directory = out/f'seg-{k:02d}'
        result = S.run_procedure(table, directory, flat(prefix, words), opts, labels, first,
                                 max_steps=10**6, cpu_limit=budget)
        words.extend(result['macro_words'])
        labels, first = directory/'labels.npz', None
        segment = dict(directory=directory.name, start=result['trajectory'][0],
                       end=dict(phi=result['phi'], digest=result['digest']),
                       macros=len(result['macro_words']), status=result['status'],
                       reason=result['reason'], cpu_s=result['cpu_s'])
        summary['segments'].append(segment)
        summary.update(phi=result['phi'], digest=result['digest'])
        save()
        if result['status'] != 'stopped':
            summary.update(status=result['status'], reason=result['reason'])
            break
        # The declared procedure stopped: search family R again without its beam.
        # The reload runs under the same guard as the search; the segment's
        # checkpoint holds the labels of this state, so the reload writes nothing.
        budget = remaining()
        if budget is not None and budget <= 0:
            summary.update(status='unknown', reason='cpu-limit')
            break
        deadline = None if budget is None else time.process_time() + budget
        try:
            with S.cpu_guard(deadline):
                state, _, _ = A.load(table, flat(prefix, words), labels)
                A.require(state.st.digest() == result['digest'], 'stop digest')
                stop = stop_search(state, stop_opts['search'], opts['depth'], opts['e0'], stop_opts['beam'],
                                   deadline)
        except S.Limit:
            summary.update(status='unknown', reason='cpu-limit')
            break
        S.atomic_json(directory/'stop-search.json', stop)
        record = stop['families']['R']
        truncated = sorted(t['target'] for t in record['targets'] if t['beam_truncated'])
        segment['stop_search'] = dict(candidates=len(record['candidates']), cpu_s=stop['cpu_s'],
                                      truncated_targets=truncated,
                                      choice=stop['choice']['word'] if stop['choice'] else None)
        save()
        if stop['choice'] is None:
            summary.update(status='stopped',
                           reason='no-candidate' if not truncated else 'no-candidate-within-beam')
            break
        first = [tuple(p) for p in stop['choice']['word']]
    else:
        summary.update(status='unknown', reason='max-segments')
    save()
    return summary


def stop_only(table, out, prefix, labels, stop_opts, depth, e0, cpu_limit=None):
    started = time.process_time()
    out = P.external_path(out)
    A.require(not (out/'stop-search.json').exists(), 'stop search exists', 'use a fresh output directory')
    A.require(cpu_limit is None or cpu_limit > 0, 'cpu-limit')
    deadline = None if cpu_limit is None else started + cpu_limit
    with S.cpu_guard(deadline):
        state, table_digest, setup = A.load(table, prefix, labels)
        stop = stop_search(state, stop_opts['search'], depth, e0, stop_opts['beam'], deadline)
    stop.update(table_digest=table_digest, prefix=[list(p) for p in prefix], stop_options=stop_opts, setup=setup)
    S.atomic_json(out/'stop-search.json', stop)
    return stop


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('table')
    ap.add_argument('out')
    ap.add_argument('--prefix', default='')
    ap.add_argument('--labels')
    ap.add_argument('--first')
    ap.add_argument('--families', default='T,R,M,E,L')
    ap.add_argument('--e0', type=int, default=20)
    ap.add_argument('--depth', type=int, default=3)
    ap.add_argument('--beam', type=int, default=64)
    ap.add_argument('--stop-search', choices=('pruned', 'beam'), default='pruned')
    ap.add_argument('--stop-beam', type=int, default=1_000_000)
    ap.add_argument('--stop-only', action='store_true')
    ap.add_argument('--cpu-limit', type=float)
    ap.add_argument('--max-segments', type=int, default=100)
    a = ap.parse_args(argv)
    P.external_path(a.out)
    if a.labels:
        P.external_path(a.labels)
    try:
        opts = S.options(a.families, a.e0, a.depth, a.beam, 'best')
        A.require(a.stop_beam >= 1, 'stop-beam')
        stop_opts = dict(search=a.stop_search, beam=a.stop_beam if a.stop_search == 'beam' else None)
        if a.stop_only:
            A.require(a.first is None, 'stop-only', '--first applies only to a run')
            stop_only(a.table, a.out, P.parse_word(a.prefix), a.labels, stop_opts, a.depth, a.e0, a.cpu_limit)
        else:
            run(a.table, a.out, P.parse_word(a.prefix), opts, stop_opts, a.labels,
                P.parse_word(a.first) if a.first is not None else None, a.cpu_limit, a.max_segments)
        return 0
    except S.Limit:
        S.atomic_json(Path(a.out)/'error.json', dict(passed=False, failures=[dict(test='cpu-limit')]))
        print('cpu-limit', file=sys.stderr)
        return 3
    except (A.Failure, ValueError, KeyError, TypeError, AssertionError) as exc:
        error = exc.record() if isinstance(exc, A.Failure) else dict(test='pstar2', detail=str(exc))
        S.atomic_json(Path(a.out)/'error.json', dict(passed=False, failures=[error]))
        print(json.dumps(error, sort_keys=True), file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
