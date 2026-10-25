"""Routing anatomy and the second analyst's bounded routing release search.

Anatomy computes obligated sets, cover opportunities and covered carrier
position sequences of one to three actions. Search keeps that cover test and
enumerates at most two helper letters, exactly as rrsearch. At every release
the driver tests the three declared prize tails in their reference order.
These weighted exact searches produce leads, not macro certificates.

Usage: python3 -I -B research/jumbling/restoration/16-six-jams/scripts/routing.py
       <seed> <out.json> anatomy <c> <r> [--word W] [--labels L]
       <seed> <out.json> search <c> <r> --nmax N --fixed g:i --free g
       --target t [--cap S] [--word W] [--labels L]
"""
import argparse
from contextlib import ExitStack
import itertools
from pathlib import Path
import sys
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as K  # noqa: E402

np, S, A = K.np, K.S, K.A


def anatomy(st, lab0, c, r, verbose=True):
    n = len(st.h)
    Ic = st.inside(c)
    inI = np.zeros(n, bool)
    inI[Ic] = True
    Sr = np.flatnonzero(lab0[:, r] == 2)
    B = {a: Ic[lab0[Ic, K.pre(c, a)[r]] == 2] for a in range(12)}
    out_r = int((~inI[Sr]).sum())
    strad_out = (lab0[~inI] == 2).any(0)
    strad_in = (lab0[Ic] == 2).any(0)
    clean0 = ~(lab0 == 2).any(0)
    cleanp = {}
    for p in range(12):
        cl = ~(strad_out | strad_in[K.pre(c, p)])
        cl[c] = cl[r] = False
        cleanp[p] = cl
    idx = {int(a): j for j, a in enumerate(Ic.tolist())}
    labIc = lab0[Ic]
    cov = {p: (labIc[:, K.pre(c, p)] == 1) & cleanp[p][None, :] for p in range(12)}

    def unc(cp, positions):
        rr = [idx[int(a)] for a in B[cp].tolist()]
        if not rr:
            return 0
        ok = np.zeros(len(rr), bool)
        for p in positions:
            ok |= cov[p][rr].any(1)
        return int((~ok).sum())

    seqs = {1: [(cp,) for cp in range(1, 12) if unc(cp, [0, cp]) == 0]}
    seqs[2] = [(a, cp) for a in range(1, 12) for cp in range(12) if cp != a and unc(cp, [0, a, cp]) == 0]
    seqs[3] = [(a, p2, cp) for a in range(1, 12) for p2 in range(12) if p2 != a for cp in range(12)
               if cp != p2 and unc(cp, [0, a, p2, cp]) == 0]
    least = {1: min(unc(cp, [0, cp]) for cp in range(1, 12)),
             2: min(unc(cp, [0, a, cp]) for a in range(1, 12) for cp in range(12) if cp != a)}
    newclean = sorted({int(y) for p in range(1, 12) for y in np.flatnonzero(cleanp[p] & ~clean0)})
    if verbose:
        print(f'[{c}->{r}] |I_c| {len(Ic)} |S_r| {len(Sr)} outside the carrier {out_r} | |B_a| '
              f'{[len(B[a]) for a in range(12)]} | newly clean helpers {newclean} | covered sequences '
              f'n=1 {len(seqs[1])} (least uncovered {least[1]}), n=2 {len(seqs[2])} of 121 (least {least[2]}),'
              f' n=3 {len(seqs[3])} of 1331', flush=True)
    return dict(c=c, r=r, B=B, cov=cov, idx=idx, seqs=seqs, out_r=out_r, newclean=newclean, least=least)


def sim(rs, word):
    stack = ExitStack()
    for g, i in word:
        if rs.ns[g] != 0:
            stack.close()
            return None
        stack.enter_context(rs.applied(g, i))
    return stack


def search(rs, an, nmax, cpu_cap, on_release, max_helpers=2, done=None):
    c, r, B, cov, idx = an['c'], an['r'], an['B'], an['cov'], an['idx']
    if done is None:
        if an['out_r']:
            return dict(tested=0, releases=0, complete=True, note='straddlers outside the carrier')
        done = lambda: rs.ns[r] == 0  # noqa: E731
    tested = releases = 0
    complete = True

    def touches(y, p, cp):
        return {int(a) for a in B[cp].tolist() if cov[p][idx[int(a)], y]}

    for nn in range(1, nmax + 1):
        for sq in an['seqs'][nn]:
            cp = sq[-1]
            pos = [0, *sq]
            items = [sq[0]] + [K.prod(c, sq[j], K.inv(c, sq[j-1])) for j in range(1, nn)]
            need = {int(a) for a in B[cp].tolist()}
            Yj = [sorted({y for y in range(600) if touches(y, p, cp)}) for p in pos]
            singles = [(j, y) for j in range(nn + 1) for y in Yj[j]]
            choices = [()] + [(s1,) for s1 in singles]
            if max_helpers >= 2:
                choices += [(s1, s2) for s1 in singles for s2 in singles
                            if s1 != s2 and (s1[0] < s2[0] or (s1[0] == s2[0] and s1[1] != s2[1]))]
            for chosen in choices:
                choice = [tuple(y for (jj, y) in chosen if jj == j) for j in range(nn + 1)]
                cover = set()
                for j, ys in enumerate(choice):
                    for y in ys:
                        cover |= touches(y, pos[j], cp)
                if not need <= cover:
                    continue
                for its in itertools.product(range(1, 12), repeat=len(chosen)):
                    if K.cpu() > cpu_cap:
                        return dict(tested=tested, releases=releases, complete=False, stopped_at=nn)
                    word, k = [], 0
                    for j in range(nn + 1):
                        if j > 0:
                            word.append((c, items[j-1]))
                        for y in choice[j]:
                            word.append((y, its[k]))
                            k += 1
                    ctx = sim(rs, word)
                    tested += 1
                    if ctx is None:
                        continue
                    with ctx:
                        if done():
                            releases += 1
                            on_release(word)
        print(f'  [{c}->{r}] n={nn} done: tested {tested}, releases {releases}, cpu {K.cpu()}', flush=True)
    return dict(tested=tested, releases=releases, complete=complete)


def run(state, mode, c, r, nmax=2, cap=300.0, fixed=None, free=-1, target=-1):
    """The routingref driver, operating on an already loaded state."""
    digest0 = state.st.digest()
    lab0 = state.lab.copy()
    an = anatomy(state, lab0, c, r)
    res = dict(digest=digest0, phi=K.J.phi(state.h), mode=mode, c=c, r=r,
               I_c=int(len(state.inside(c))), S_r=int((lab0[:, r] == 2).sum()), out_r=an['out_r'],
               B=[len(an['B'][a]) for a in range(12)], newclean=an['newclean'],
               seqs={str(n): [list(s) for s in an['seqs'][n]] for n in (1, 2, 3)},
               least={str(n): an['least'][n] for n in (1, 2)}, complete=True)
    if mode == 'search':
        A.require(1 <= nmax <= 3, 'nmax')
        A.require(fixed is not None and len(fixed) == 2, 'fixed')
        K.P.grip(fixed[0])
        A.require(1 <= fixed[1] < 12, 'fixed retained item')
        K.P.grip(free)
        K.P.grip(target)
        rs = S.RetainedSearch(state)
        hits = []

        def on_release(word):
            best = None
            for seq_fn in (lambda m: [fixed, (free, m)], lambda m: [(free, m), fixed], lambda m: [(free, m)]):
                for m in range(1, 12):
                    ctx = sim(rs, seq_fn(m))
                    if ctx is None:
                        continue
                    with ctx:
                        s_, f_, ds = rs.metric(target)
                        if s_ == 0:
                            v = (f_ - int(ds.max()), f_, int(ds.max()), seq_fn(m))
                            best = v if best is None or v[:3] < best[:3] else best
            hits.append((best, list(word)))

        sres = search(rs, an, nmax, K.cpu() + cap, on_release)
        clear = sorted([h for h in hits if h[0] is not None], key=lambda t: t[0][:3])
        res.update(nmax=nmax, cap=cap, fixed=list(fixed), free=free, target=target, search=sres,
                   releases_listed=len(hits), clearable=len(clear), complete=sres['complete'],
                   best=[dict(value=list(h[0][:3]), tail=[list(e) for e in h[0][3]],
                              word=','.join(f'{a}:{b}' for a, b in h[1])) for h in clear[:5]])
    A.require(state.st.digest() == digest0, 'undo', 'routing')
    res['cpu_s'] = K.cpu()
    return res


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('seed')
    ap.add_argument('out')
    ap.add_argument('mode', choices=('anatomy', 'search'))
    ap.add_argument('c', type=int)
    ap.add_argument('r', type=int)
    ap.add_argument('--nmax', type=int, default=2)
    ap.add_argument('--fixed')
    ap.add_argument('--free', type=int, default=-1)
    ap.add_argument('--target', type=int, default=-1)
    ap.add_argument('--cap', type=float, default=300.0)
    ap.add_argument('--word', default='')
    ap.add_argument('--labels')
    a = ap.parse_args(argv)
    started = time.process_time()
    state = K.load(a.seed, a.word, a.labels)
    opts = {k: v for k, v in vars(a).items() if k not in ('seed', 'out')}
    res = K.record('routing', a.seed, state, a.word, opts)
    fixed = K.P.parse_word(a.fixed) if a.fixed else []
    A.require(len(fixed) <= 1, 'fixed', 'expected one retained letter')
    res.update(run(state, a.mode, K.P.grip(a.c), K.P.grip(a.r), a.nmax, a.cap,
                   fixed[0] if fixed else None, a.free, a.target))
    # routingref used an integer seed; keep its field type.
    res.update(seed=int(a.seed), cpu_s=round(time.process_time()-started, 1))
    K.atomic_json(a.out, res)
    print('done', {k: v for k, v in res.items() if k != 'seqs'}, flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
