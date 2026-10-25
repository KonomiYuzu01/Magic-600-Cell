"""Exact reference T1 scan with one leaf: leaf jumble, lowering, optional return.

Every leaf with delta <= E is applied exactly. Touched clean targets are read
from their exact budgets. Only the best lowering per leaf (least pair total,
then target grip) is followed by a return, and only when that lowering
descends after the leaf. This is the scratch t1scan enumeration, not a scan of
all possible three-letter words. Every applied letter is undone.

Usage: python3 -I -B research/jumbling/restoration/16-six-jams/scripts/t1.py
       <seed> <out.json> [--word W] [--labels L] [--emax 900]
       [--shard i/n] [--grips g1,g2,...]
"""
import argparse
from pathlib import Path
import sys
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as C  # noqa: E402

P, A, np = C.P, C.A, C.np


def scan(state, emax=900, shard=(0, 1), grips=(), progress=None, verbose=True):
    """Return the reference fields; progress receives atomic checkpoint data."""
    t0 = time.process_time()
    shard, nshard = shard
    A.require(nshard >= 1 and 0 <= shard < nshard, 'shard')
    only = list(grips)
    digest0, phi0 = state.st.digest(), P.J.phi(state.h)
    clean0 = ~(state.lab == 2).any(0)
    firsts = []
    for y in np.flatnonzero(clean0).tolist():
        if only and y not in only:
            continue
        f, ds = state.budget(y)
        for h, d in zip(P.JUMBLES, ds.tolist()):
            if 2*(f-d) <= emax:
                firsts.append((2*(f-d), y, h))
    firsts.sort()
    mine = [t for k, t in enumerate(firsts) if k % nshard == shard]
    if verbose:
        print(f'shard {shard}/{nshard}: phi {phi0}, {int(clean0.sum())} clean grips, {len(firsts)} leaves with '
              f'dPhi <= {emax}, {len(mine)} in this shard (cpu {time.process_time()-t0:.0f})', flush=True)
    hits, tested, returns, best2, best3, per_leaf = [], 0, 0, None, None, []

    def snapshot(complete):
        hs = sorted(hits, key=lambda r: (r['terminal_phi'], r['excursion']))
        return dict(digest=digest0, phi=phi0, emax=emax, grips=only, shard=[shard, nshard],
                    leaves=len(mine), leaves_done=len(per_leaf), leaves_all=len(firsts), targets_tested=tested,
                    returns=returns, complete=complete, best_pair=best2, best_return=best3, hits=hs[:50],
                    n_hits=len(hs), per_leaf=per_leaf, cpu_s=round(time.process_time()-t0, 1))

    for k, (dy, y, h) in enumerate(mine):
        _, tok = state.apply(y, h, record=False)
        A.require(tok is not None, 'leaf admissibility')
        try:
            moved, _, _, lab_old = tok
            lab_new = state.lab[moved]
            touched = ((lab_old != 0) | (lab_new != 0)).any(0)
            touched[y] = False
            lead = None
            for x in np.flatnonzero(touched).tolist():
                if (state.lab[:, x] == 2).any():
                    continue
                tested += 1
                f, ds = state.budget(x)
                d = int(ds.max())
                total = dy + 2*(f-d)
                g = [i for i, v in zip(P.JUMBLES, ds.tolist()) if v == d][0]
                if best2 is None or total < best2[0]:
                    best2 = [total, [y, h], [x, g], f, d]
                if lead is None or (total, x) < (lead[0], lead[1]):
                    lead = (total, x, g, f, d)
                if total < 0:
                    hits.append(dict(word=[[y, h], [x, g]], a=dy, F=f, D=d,
                                     terminal_phi=phi0 + total, excursion=dy))
            leaf = dict(leaf=[y, h], a=dy, lead=list(lead) if lead else None)
            if lead is not None and lead[0] < dy:
                total2, x, g = lead[0], lead[1], lead[2]
                _, tok2 = state.apply(x, g, record=False)
                A.require(tok2 is not None, 'lowering admissibility')
                try:
                    if not (state.lab[:, y] == 2).any():
                        returns += 1
                        f, ds = state.budget(y)
                        d = int(ds.max())
                        hp = [i for i, v in zip(P.JUMBLES, ds.tolist()) if v == d][0]
                        total3 = total2 + 2*(f-d)
                        leaf['return'] = [hp, f, d, total3]
                        if best3 is None or total3 < best3[0]:
                            best3 = [total3, [y, h], [x, g], [y, hp]]
                        if total3 < 0:
                            hits.append(dict(word=[[y, h], [x, g], [y, hp]], a=dy, pair_total=total2, F=f, D=d,
                                             terminal_phi=phi0 + total3, excursion=max(dy, total2)))
                    else:
                        leaf['return'] = 'blocked'
                finally:
                    state.undo(x, g, tok2)
            per_leaf.append(leaf)
        finally:
            state.undo(y, h, tok)
        if verbose and k % 50 == 49:
            print(f'  {k+1} of {len(mine)} leaves, {tested} targets, {returns} returns, {len(hits)} hits, '
                  f'best pair {best2}, best return {best3} (cpu {time.process_time()-t0:.0f})', flush=True)
        if progress and k % 10 == 9 and k + 1 < len(mine):
            progress(snapshot(False))
    A.require(state.st.digest() == digest0, 'undo', 'T1')
    return snapshot(True)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('seed')
    ap.add_argument('out')
    ap.add_argument('--word', default='')
    ap.add_argument('--labels')
    ap.add_argument('--emax', type=int, default=900)
    ap.add_argument('--shard', default='0/1')
    ap.add_argument('--grips', default='')
    a = ap.parse_args(argv)
    started = time.process_time()
    state = C.load(a.seed, a.word, a.labels)
    opts = {k: v for k, v in vars(a).items() if k not in ('seed', 'out')}
    base = C.record('t1', a.seed, state, a.word, opts)

    def save(row):
        C.atomic_json(a.out, dict(base, extra=a.word, **dict(row, cpu_s=round(time.process_time()-started, 1))))

    print(f'{a.seed} ', end='', flush=True)
    result = scan(state, a.emax, tuple(int(v) for v in a.shard.split('/')), C.grips(a.grips), save)
    save(result)
    print(f'done: {result["n_hits"]} hits, best pair {result["best_pair"]}, best return '
          f'{result["best_return"]}, cpu {time.process_time()-started:.1f}', flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
