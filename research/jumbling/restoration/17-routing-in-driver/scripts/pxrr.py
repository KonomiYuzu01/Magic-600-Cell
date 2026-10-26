"""P***: P** (stage 16 `pxrun.py`) with family RR in its stop search.

P* segments use T,R,M,E,L, E0 20, depth 3, beam 64. A stop searches R without
a beam, X1 and X2, then minimal X3 when X1 and X2 found nothing, as in P**;
when R and X found nothing, it searches family RR (`rr.py`) with at most two
carrier actions and at most `helpers` helper letters, and with three carrier
actions only when those found nothing and nmax is 3. The best candidate is
certified again by the next segment before it is committed.
Input caches stay read-only: S15 receives only output-owned label copies.

Usage: python3 -I -B research/jumbling/restoration/17-routing-in-driver/scripts/pxrr.py
       <seed> <out dir> <cpu limit s> [--labels L] [--start "W1;W2;..."]
       [--nmax 2] [--helpers 2]
"""
import argparse
from pathlib import Path
import sys
import time

sys.dont_write_bytecode = True
_HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(_HERE), str(_HERE.parents[1]/'16-six-jams'/'scripts')]
import common as C  # noqa: E402
import transport as X  # noqa: E402
import pstar2 as S2  # noqa: E402
import rr as RR  # noqa: E402

S, P, A = C.S, C.P, C.A


def run(seed, out, cpu_limit, labels=None, start='', nmax=2, helpers=2):
    out = P.external_path(out)
    # This refusal precedes every table/cache load, even for a malformed seed.
    A.require(not (out/'pxrr.json').exists(), 'run exists', 'use a fresh output directory')
    A.require(cpu_limit > 0, 'cpu-limit')
    A.require(1 <= nmax <= 3 and 0 <= helpers <= 3, 'RR bounds')
    started = time.process_time()
    start_macros = [P.parse_word(w) for w in start.split(';')] if start else []
    extra = ','.join(f'{c}:{i}' for m in start_macros for c, i in m)
    rec = C.jam(seed)
    prefix = P.parse_word(C.jam_word(seed))
    table = rec['table']
    opts = S.options('T,R,M,E,L', 20, 3, 64, 'best')
    out.mkdir(parents=True, exist_ok=True)
    # Even a valid supplied cache is copied. S14 may write through a cache
    # argument, so no caller-owned filename is ever passed to S15.
    owned_labels = out/'start-labels.npz'
    state = C.load(seed, extra, labels, save_labels=owned_labels)
    summary = C.record('pxrr', seed, state, extra,
                       dict(cpu_limit=cpu_limit, labels=str(labels) if labels is not None else None,
                            start=start, procedure=opts, rr=dict(nmax=nmax, helpers=helpers)))
    summary.update(start=dict(digest=state.st.digest(), phi=C.J.phi(state.h)), segments=[],
                   start_macros=[[list(e) for e in m] for m in start_macros],
                   status='unknown', reason='initialised', complete=False)
    words, first, labels = [list(m) for m in start_macros], None, owned_labels
    del state

    def save():
        summary['cpu_s'] = round(time.process_time()-started, 3)
        C.atomic_json(out/'pxrr.json', summary)

    for k in range(1000):
        budget = cpu_limit - (time.process_time()-started)
        if budget <= 0:
            summary.update(status='unknown', reason='cpu-limit')
            break
        directory = out/f'seg-{k:03d}'
        result = S.run_procedure(table, directory, S2.flat(prefix, words), opts, labels, first,
                                 max_steps=10**6, cpu_limit=budget)
        words.extend(result['macro_words'])
        labels, first = directory/'labels.npz', None
        seg = dict(directory=directory.name, macros=len(result['macro_words']), status=result['status'],
                   end=dict(phi=result['phi'], digest=result['digest']),
                   families=[s.get('choice', {}).get('family') if s.get('choice') else None
                             for s in result['steps']], words=result['macro_words'], cpu_s=result['cpu_s'])
        summary['segments'].append(seg)
        summary.update(phi=result['phi'], digest=result['digest'])
        save()
        print(f"seg {k}: {seg['macros']} macros {seg['families']} -> phi {result['phi']} ({result['status']})",
              flush=True)
        if result['status'] != 'stopped':
            summary.update(status=result['status'], reason=result['reason'], complete=result['status'] == 'depth-one')
            break
        t0 = time.process_time()
        state, _, _ = A.load(table, S2.flat(prefix, words), labels)
        A.require(state.st.digest() == result['digest'], 'stop digest')
        stop = S2.stop_search(state, 'pruned', 3, 20, None)
        cands = [dict(c, family='R') for c in stop['families']['R']['candidates']]
        xrec = []

        def add_x(K, minimal=False):
            rows, counts = X.search(state, K, minimal)
            xrec.append(counts)
            cands.extend(dict(S.raw_candidate(r['word'], r['terminal_phi'], 0), F=r['F'], D=r['D'],
                              family=f'X{K}') for r in rows)

        for K in (1, 2):
            add_x(K)
        if not any(c['family'].startswith('X') for c in cands):
            add_x(3, minimal=True)
        seg['stop'] = dict(R=len(stop['families']['R']['candidates']), X=xrec)
        # RR is searched only where R and X (K <= 3) have no candidate: first
        # with at most two carrier actions, then with three only when those
        # words gave no candidate, as X3 follows X1 and X2.
        seg['stop']['RR'] = []
        for lo, hi in ((1, min(2, nmax)), (3, 3)):
            if cands or lo > nmax:
                break
            rows, pairs, counts = RR.search(state, hi, helpers, nmin=lo)
            seg['stop']['RR'].append(dict(counts=counts, pairs=[{k: v for k, v in p.items() if k != 'best'}
                                                                for p in pairs],
                                          best=[dict(word=r['word'], terminal_phi=r['terminal_phi'],
                                                     target=r['target'], carrier=r['carrier'],
                                                     released=r['released'], F=r['F'], D=r['D'])
                                                for r in rows[:20]]))
            cands.extend(dict(r, family=f'RR{hi}') for r in rows)
        seg['stop']['cpu_s'] = round(time.process_time()-t0, 1)
        if not cands:
            summary.update(status='stopped', reason='no-candidate', complete=True)
            save()
            print('stop: no candidate in R, X (K <= 3) or RR', flush=True)
            break
        choice = min(cands, key=S.candidate_key)
        seg['stop']['choice'] = dict(word=choice['word'], terminal_phi=choice['terminal_phi'], family=choice['family'])
        save()
        print(f"  stop search: {choice['family']} {choice['word']} -> {choice['terminal_phi']}"
              f" (cpu {seg['stop']['cpu_s']})", flush=True)
        first = [tuple(p) for p in choice['word']]
    save()
    print('done', summary.get('status'), summary.get('reason'), 'phi', summary.get('phi'),
          'cpu', summary['cpu_s'], flush=True)
    return summary


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('seed')
    ap.add_argument('out')
    ap.add_argument('cpu_limit', type=float)
    ap.add_argument('--labels')
    ap.add_argument('--start', default='')
    ap.add_argument('--nmax', type=int, default=2)
    ap.add_argument('--helpers', type=int, default=2)
    a = ap.parse_args(argv)
    run(a.seed, a.out, a.cpu_limit, a.labels, a.start, a.nmax, a.helpers)
    return 0


if __name__ == '__main__':
    sys.exit(main())
