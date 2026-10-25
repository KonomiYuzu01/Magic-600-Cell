"""Family X: exactly K retained letters, then a lowering at a dynamic target.

The reference's adjacent-grip reductions, minimal leaf restriction, counters
and lexicographic tie-breaks are retained. --minimal assumes that no shorter
retained word descends; this tool does not establish that premise. Hits are
leads for macro.py cert, not certified macros.

Usage: python3 -I -B research/jumbling/restoration/16-six-jams/scripts/transport.py
       <seed> <K> <out.json> [--minimal] [--word W] [--labels L]
"""
import argparse
from pathlib import Path
import sys
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as C  # noqa: E402

S, P, A, np, CLS = C.S, C.P, C.A, C.np, C.CLS
_NBR = None


def nbr():
    global _NBR
    if _NBR is None:
        _NBR = [set(p) | {c} for c, p in enumerate(P.graph()[2])]
    return _NBR


def search(state, K, minimal=False, deadline=None):
    """Return (best rows per target, counts), as in the scratch xsearch.

    Rows use dynk's word, terminal_phi, F, D, target and ordering. Function
    counts also carry K and minimal, as required by the P* stop records.
    deadline is an absolute process_time deadline, as in pstar.tick.
    """
    A.require(type(K) is int and K >= 1, 'K')
    rs = S.RetainedSearch(state)
    phi0 = P.J.phi(state.h)
    near = nbr()
    lab_root = rs.lab.copy()
    m_cache, hits = {}, {}
    counts = dict(K=K, minimal=minimal, nodes=0, letters=0, leaf_tests=0, hits=0,
                  skipped_same=0, skipped_commute=0, skipped_minimal=0)

    def m_root(c):
        if c not in m_cache:
            moved = np.flatnonzero(lab_root[:, c] == 1)
            m_cache[c] = (lab_root[moved] == 2).T @ rs.w[moved]
        return m_cache[c]

    def m_node(c, changed):
        base = m_root(c)
        if not len(changed):
            return base
        was, now = lab_root[changed, c] == 1, rs.lab[changed, c] == 1
        if not (was.any() or now.any()):
            return base
        return (base + (rs.lab[changed[now]] == 2).T @ rs.w[changed[now]]
                - (lab_root[changed[was]] == 2).T @ rs.w[changed[was]])

    def record(events, x, f, ds):
        for i, d in zip(P.JUMBLES, ds.tolist()):
            if f < d:
                counts['hits'] += 1
                row = dict(word=[list(e) for e in events] + [[x, i]],
                           terminal_phi=phi0 + 2*(f-d), F=f, D=d, target=x)
                key = (row['terminal_phi'], row['word'])
                if x not in hits or key < hits[x][0]:
                    hits[x] = (key, row)

    def leaves(prefix, c, changed, cache):
        S.tick(deadline)
        moved = np.flatnonzero(rs.lab[:, c] == 1)
        m2 = m_node(c, changed)
        hr = np.flatnonzero(rs.sec >= 0)
        inside = np.isin(hr, moved)
        h_out, h_in = hr[~inside], hr[inside]
        hold_out = ((rs.lab[h_out] == 1) & (rs.b[h_out][:, None] == CLS[None, :])).any(0) if len(h_out) else \
            np.zeros(600, bool)
        allowed = np.ones(600, bool)
        if minimal:
            gs = [g for g, _ in prefix] + [c]
            for j, g in enumerate(gs[:-1]):
                if not any(h in near[g] for h in gs[j+1:]):
                    mask = np.zeros(600, bool)
                    mask[list(near[g])] = True
                    allowed &= mask
            mask = np.zeros(600, bool)
            mask[list(near[c])] = True
            allowed &= mask
        for i in range(1, 12):
            counts['letters'] += 1
            _, pre, _, sigma = A.retained(c, i)
            ns_after = rs.ns - m2 + m2[pre]
            hold = hold_out.copy()
            if len(h_in):
                hold |= ((rs.lab[h_in][:, pre] == 1) & (sigma[rs.b[h_in]][:, None] == CLS[None, :])).any(0)
            for x in np.flatnonzero((ns_after == 0) & hold & allowed).tolist():
                counts['leaf_tests'] += 1
                if x not in cache:
                    cache[x] = rs.metric(x)
                s, f, ds = rs.leaf(x, c, i, cache[x], moved)
                A.require(s == 0, 'leaf clean')
                record((*prefix, (c, i)), x, f, ds)

    def next_grips(prefix):
        last = prefix[-1][0] if prefix else None
        for c in np.flatnonzero(rs.ns == 0).tolist():
            if last is not None:
                if c == last:
                    counts['skipped_same'] += 11
                    continue
                if c < last and c not in near[last]:
                    counts['skipped_commute'] += 11
                    continue
            yield c

    def expand(prefix, changed):
        counts['nodes'] += 1
        if len(prefix) == K - 1:
            cache = {}
            for c in next_grips(prefix):
                leaves(prefix, c, changed, cache)
            return
        for c in next_grips(prefix):
            moved = np.flatnonzero(rs.lab[:, c] == 1)
            now = np.union1d(changed, moved)
            for i in range(1, 12):
                with rs.applied(c, i):
                    expand((*prefix, (c, i)), now)

    digest = state.st.digest()
    expand((), np.zeros(0, np.int64))
    A.require(state.st.digest() == digest, 'undo', 'family X')
    return sorted((r for _, r in hits.values()), key=lambda r: (r['terminal_phi'], r['word'])), counts


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('seed')
    ap.add_argument('K', type=int)
    ap.add_argument('out')
    ap.add_argument('--minimal', action='store_true')
    ap.add_argument('--word', default='')
    ap.add_argument('--labels')
    a = ap.parse_args(argv)
    started = time.process_time()
    state = C.load(a.seed, a.word, a.labels)
    rows, counts = search(state, a.K, a.minimal)
    result = C.record('transport', a.seed, state, a.word,
                      dict(K=a.K, minimal=a.minimal, word=a.word, labels=a.labels))
    result.update(extra_prefix=a.word, K=a.K, minimal=a.minimal,
                  counts={k: v for k, v in counts.items() if k not in ('K', 'minimal')},
                  best_per_target=rows, complete=True, cpu_s=round(time.process_time()-started, 3))
    C.atomic_json(a.out, result)
    print(a.seed, 'K', a.K, 'minimal', a.minimal, 'hits', counts['hits'], 'targets', len(rows),
          'nodes', counts['nodes'], 'cpu', result['cpu_s'], flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
