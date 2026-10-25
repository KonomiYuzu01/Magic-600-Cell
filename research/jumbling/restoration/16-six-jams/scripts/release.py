"""Routing releases with a prize, preserving the scratch rrp beam search.

Prizes formally read a blocked grip's present inside set. Carriers cover its
straddlers, and helpers can hold pieces a carrier puts across it. The beam is
scored by (min(S_r, least next carrier S_r), S_r). A released grip is tested
at every prize target. Hits are leads for macro.py cert; the beam is never an
exhaustive search, including when it finds a hit.

Usage: python3 -I -B research/jumbling/restoration/16-six-jams/scripts/release.py
       <seed> <out.json> [--word W] [--labels L] [--width 2000] [--depth 10]
       [--prizes 6] [--only-r R,...] [--dedup] [--alphabet a,b,...]
       [--relevant] [--enable]
"""
import argparse
import hashlib
from pathlib import Path
import sys
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as C  # noqa: E402

S, P, A, np, CLS = C.S, C.P, C.A, C.np, C.CLS


def scan(state, width=2000, depth=10, nprize=6, only_r=(), dedup=False,
         forced=(), relevant_only=False, enable=False, progress=None, verbose=True):
    """The reference enumeration, including generation order and stable sorts."""
    A.require(width >= 1 and depth >= 1 and nprize >= 0, 'beam options')
    t0 = time.process_time()
    digest0, phi0 = state.st.digest(), P.J.phi(state.h)
    rs = S.RetainedSearch(state)
    partners = P.graph()[2]
    pre = {}

    def prem(g, i):
        if (g, i) not in pre:
            pre[(g, i)] = np.asarray(A.retained(g, i)[1])
        return pre[(g, i)]

    clean = rs.ns == 0
    hold_own = ((rs.lab == 1) & (rs.sec >= 0)[:, None] & (rs.b[:, None] == CLS[None, :])).any(0)
    prizes = []
    for r in np.flatnonzero(~clean).tolist():
        moved = np.flatnonzero(rs.lab[:, r] == 1)
        targets = [x for x in partners[r] if hold_own[x] or (rs.lab[moved][:, x] == 1).any()]
        for x in targets:
            base = rs.metric(x)
            for m in range(1, 12):
                s, f, ds = rs.leaf(x, r, m, base, moved)
                d = int(ds.max())
                if s == 0 and f < d:
                    prizes.append(dict(r=r, m=m, x=x, F=f, D=d, value=2*(d-f), S_r=int(rs.ns[r])))
    best_by_r = {}
    for p in prizes:
        if p['r'] not in best_by_r or p['value'] > best_by_r[p['r']]['value']:
            best_by_r[p['r']] = p
    ranked = sorted(best_by_r.values(), key=lambda p: -p['value'])
    if verbose:
        print(f'phi {phi0}, {int(clean.sum())} clean grips, {len(prizes)} prize letters at {len(best_by_r)} '
              f'blocked grips; best: {[(p["r"], p["x"], p["value"], p["S_r"]) for p in ranked[:10]]}'
              f' (cpu {time.process_time()-t0:.0f})', flush=True)

    def s_after(g, r):
        moved = np.flatnonzero(rs.lab[:, g] == 1)
        col = rs.lab[moved] == 2
        w = rs.w[moved]
        base = int(rs.ns[r]) - int(w[col[:, r]].sum())
        return np.array([base + int(w[col[:, prem(g, i)[r]]].sum()) for i in range(12)])

    enabled = {}
    if enable:
        for y in np.flatnonzero(clean).tolist():
            cap = np.flatnonzero(rs.lab[:, y] == 1)
            m_y = (rs.lab[cap] == 2).T @ rs.w[cap]
            for a in range(1, 12):
                ns_a = rs.ns - m_y + m_y[prem(y, a)]
                for g in np.flatnonzero((ns_a == 0) & ~clean).tolist():
                    enabled.setdefault(g, []).append((y, a))
        if verbose:
            print(f'  {len(enabled)} grips are enabled by one retained letter', flush=True)

    results = []

    def snapshot():
        return dict(prizes=ranked[:40], results=results, clean=int(clean.sum()),
                    prize_letters=len(prizes), prize_grips=len(best_by_r), complete=False,
                    search='beam', cpu_s=round(time.process_time()-t0, 1))

    for p in ranked:
        if len(results) >= nprize:
            break
        r = p['r']
        if only_r and r not in only_r:
            continue
        strad = np.flatnonzero(rs.lab[:, r] == 2)
        pool_g = np.flatnonzero(clean).tolist() + [g for g in enabled if g != r]
        carriers = sorted(c for c in pool_g if (rs.lab[strad, c] == 1).any())
        enablers = sorted({y for c in carriers if c in enabled for y, _ in enabled[c]})
        covered = np.zeros(len(strad), bool)
        for c in carriers:
            covered |= rs.lab[strad, c] == 1
        if not covered.all():
            results.append(dict(prize=p, status='level above 1', carriers=carriers, n_hits=0))
            continue
        helpers = set()
        for c in carriers:
            cap = np.flatnonzero(rs.lab[:, c] == 1)
            L = rs.lab[cap]
            bad = np.zeros(len(cap), bool)
            for k in range(12):
                bad |= L[:, prem(c, k)[r]] == 2
            Lb = L[bad]
            m_c = (L == 2).T @ rs.w[cap]
            for a in range(12):
                pa = prem(c, a)
                clean_a = (rs.ns - m_c + m_c[pa]) == 0
                for y in np.flatnonzero(clean_a).tolist():
                    if y in carriers or y == r:
                        continue
                    if (Lb[:, pa[y]] == 1).any():
                        helpers.add(y)
        alphabet = forced or sorted(set(carriers) | helpers | set(enablers))
        targets = sorted({q['x'] for q in prizes if q['r'] == r})
        t1 = time.process_time()
        beam, seen, hits, releases = [()], set(), [], 0
        U = np.flatnonzero((rs.lab[:, alphabet] == 1).any(1))
        states = set()
        best_score = None
        for level in range(1, depth + 1):
            scored = []
            for wd in beam:
                with rs.path(wd):
                    last = wd[-1][0] if wd else None
                    if relevant_only:
                        need = rs.lab[:, r] == 2
                        for c in carriers:
                            if rs.ns[c] == 0:
                                inside = rs.lab[:, c] == 1
                                for k in range(1, 12):
                                    need |= inside & (rs.lab[:, prem(c, k)[r]] == 2)
                        blocked = [g for g in alphabet if rs.ns[g] != 0]
                        if blocked:
                            need |= (rs.lab[:, blocked] == 2).any(1)
                    for g in alphabet:
                        if g == last or rs.ns[g] != 0:
                            continue
                        if relevant_only and g not in carriers and not (need & (rs.lab[:, g] == 1)).any():
                            continue
                        sa = s_after(g, r)
                        for i in range(1, 12):
                            scored.append((int(sa[i]), wd + ((g, i),)))
            scored.sort(key=lambda t: t[0])
            pool = []
            for s, wd in scored[:(20 if dedup else 3) * width]:
                if wd in seen:
                    continue
                if dedup and len(pool) >= 3 * width:
                    break
                seen.add(wd)
                with rs.path(wd):
                    if dedup:
                        key = hashlib.blake2b(rs.lab[U].tobytes() + rs.b[U].tobytes(), digest_size=16).digest()
                        if key in states:
                            continue
                        states.add(key)
                    la = int(rs.ns[r])
                    if s:
                        for c in carriers:
                            if rs.ns[c] == 0:
                                la = min(la, int(s_after(c, r)[1:].min()))
                    else:
                        releases += 1
                        moved = np.flatnonzero(rs.lab[:, r] == 1)
                        for x in targets:
                            base = rs.metric(x)
                            for m in range(1, 12):
                                sx, f, ds = rs.leaf(x, r, m, base, moved)
                                d = int(ds.max())
                                if sx == 0 and f < d:
                                    hits.append(dict(word=[list(e) for e in wd] + [[r, m]], x=x, F=f, D=d,
                                                     terminal_phi=phi0 + 2*(f-d)))
                pool.append(((min(s, la), s), wd))
            pool.sort(key=lambda t: t[0])
            beam = [w for _, w in pool[:width]]
            best_score = pool[0][0] if pool else None
            if hits:
                break
        hits.sort(key=lambda h: (h['terminal_phi'], len(h['word'])))
        res = dict(prize=p, carriers=carriers, helpers=sorted(helpers), levels=level, best_score=best_score,
                   releases=releases, n_hits=len(hits), hits=hits[:20], cpu_s=round(time.process_time()-t1, 1))
        results.append(res)
        if verbose:
            print(f'  r {r} (S {p["S_r"]}, prize {p["value"]} at {p["x"]}): carriers {carriers}, '
                  f'helpers {sorted(helpers)}; levels {level}, best score {best_score}, releases {releases}, '
                  f'hits {len(hits)}' + (f', best {hits[0]["terminal_phi"]} by {hits[0]["word"]}' if hits else '')
                  + f' (cpu {res["cpu_s"]})', flush=True)
        if progress:
            progress(snapshot())
    A.require(state.st.digest() == digest0, 'undo', 'routing release')
    return snapshot()


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('seed')
    ap.add_argument('out')
    ap.add_argument('--word', default='')
    ap.add_argument('--labels')
    ap.add_argument('--width', type=int, default=2000)
    ap.add_argument('--depth', type=int, default=10)
    ap.add_argument('--prizes', type=int, default=6)
    ap.add_argument('--only-r', default='')
    ap.add_argument('--dedup', action='store_true')
    ap.add_argument('--alphabet', default='')
    ap.add_argument('--relevant', action='store_true')
    ap.add_argument('--enable', action='store_true')
    a = ap.parse_args(argv)
    started = time.process_time()
    state = C.load(a.seed, a.word, a.labels)
    opts = {k: v for k, v in vars(a).items() if k not in ('seed', 'out')}
    base = C.record('release', a.seed, state, a.word, opts)

    def save(row):
        C.atomic_json(a.out, dict(base, extra=a.word, **dict(row, cpu_s=round(time.process_time()-started, 1))))

    print(f'{a.seed}: ', end='', flush=True)
    result = scan(state, a.width, a.depth, a.prizes, C.grips(a.only_r), a.dedup,
                  C.grips(a.alphabet), a.relevant, a.enable, save)
    save(result)
    print('done', round(time.process_time()-started, 1), flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
