"""Family RR: routing releases with short tails (Lemma RR), for the stop search of P***.

At a configuration Z, for every blocked grip r and every grip c clean at Z that
holds every straddler of r, the routing words are the carrier position
sequences of at most nmax actions at c that pass the cover test (Lemma OT), with
at most max_helpers helper letters at cover opportunities, every retained item
and every admissible word. Two reductions keep one word per configuration:
- Canonical words. Two carrier letters with no helper between them compose to
  one carrier letter, so a word whose middle position has no helper reaches the
  configuration of a shorter word with the same helpers; that word passes the
  cover test with the same helper positions and is enumerated. Only words with
  a helper at every middle position are built.
- Equal configurations. Words that reach the same configuration of the search
  rows share their tails; the tails are searched once and the shortest word
  (then the least) is kept.
After a word that releases r (no straddler at r), the tails are the words of
at most two retained letters, at most one of them at a grip other than r,
followed by a lowering (x, j) at a grip x that is clean and holds a high piece of
its own branch (Lemma RR with at most one letter u besides the releaser), in the
forms that the reductions leave:
  (a) (x, j)              a lowering right after the routing word;
  (b) u (x, j)            u at a clean grip y other than r, x a partner of y;
  (c) (r, m) (x, j)       x a partner of r;
  (d) (r, m) u (x, j)     u at a clean grip y, x a partner of y, and r a partner
                          of y or of x;
  (e) u (r, m) (x, j)     u at a clean partner of r, x a partner of r.
A tail letter that is not a partner of a later letter or of the target commutes
past them and leaves the target's F and D unchanged (the minimality reduction
R3), so removing it gives a shorter word of the same language with the same
terminal phi; in (e) a letter u that is not a partner of r commutes with the
releaser (form d); two letters at one grip in a row are one letter. Hence the
best candidate per target, by terminal phi and then length, is that of the whole
language. All counts are exact reads of the search rows; hits are leads that the
driver certifies before it commits them.

Usage: python3 -I -B research/jumbling/restoration/17-routing-in-driver/scripts/rr.py
       <seed> <out.json> [--word W] [--labels L] [--nmin 1] [--nmax 2] [--helpers 2]
       [--pairs c:r,...] [--count-only]
"""
import argparse
from functools import lru_cache
import hashlib
import itertools
from pathlib import Path
import sys
import time

sys.dont_write_bytecode = True
_HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(_HERE), str(_HERE.parents[1]/'16-six-jams'/'scripts')]
import common as C  # noqa: E402
import routing as RT  # noqa: E402
import transport as X  # noqa: E402

S, P, A, J, np, CLS = C.S, C.P, C.A, C.J, C.np, C.CLS


@lru_cache(maxsize=None)
def codes(x):
    return S.seconds().codes(x)


def metric(rs, x):
    """RetainedSearch.metric with the second-vertex codes of x cached."""
    inside = rs.lab[:, x] == 1
    f = int(rs.w[inside & (rs.b != CLS[x])].sum())
    high = inside & (rs.sec >= 0)
    dc = np.zeros(20, np.int64)
    np.add.at(dc, rs.sec[high], rs.w[high])
    return int(rs.ns[x]), f, dc[codes(x)]


def mask(grips):
    m = np.zeros(600, bool)
    m[list(grips)] = True
    return m


def routing_words(an, nmax, max_helpers, canonical=True, nmin=1):
    """Canonical routing words of the cover test, in a fixed order.

    canonical=False also yields the words with a middle position without a
    helper (the unreduced language of stage 16 `routing.search`), for checks.
    """
    c, B, cov, idx = an['c'], an['B'], an['cov'], an['idx']
    for nn in range(nmin, nmax + 1):
        for sq in an['seqs'][nn]:
            cp = sq[-1]
            pos = [0, *sq]
            items = [sq[0]] + [C.prod(c, sq[j], C.inv(c, sq[j-1])) for j in range(1, nn)]
            need = {int(a) for a in B[cp].tolist()}
            rows = [idx[a] for a in sorted(need)]
            touch = {}
            for j, p in enumerate(pos):
                if rows:
                    hit = cov[p][rows]
                    for y in np.flatnonzero(hit.any(0)).tolist():
                        touch[(j, y)] = {a for a, h in zip(sorted(need), hit[:, y].tolist()) if h}
            singles = sorted(touch)
            for h in range(max_helpers + 1):
                for chosen in itertools.permutations(singles, h):
                    if any(chosen[k][0] > chosen[k+1][0] for k in range(h - 1)):
                        continue
                    at = {j for j, _ in chosen}
                    if canonical and any(j not in at for j in range(1, nn)):
                        continue
                    cover = set().union(*(touch[s] for s in chosen)) if chosen else set()
                    if not need <= cover:
                        continue
                    for its in itertools.product(range(1, 12), repeat=h):
                        word, k = [], 0
                        for j in range(nn + 1):
                            if j > 0:
                                word.append((c, items[j-1]))
                            for jj, y in chosen:
                                if jj == j:
                                    word.append((y, its[k]))
                                    k += 1
                        yield nn, word


class Walker:
    """Apply words one after another, keeping the longest common prefix applied."""

    def __init__(self, rs):
        self.rs, self.stack = rs, []

    def go(self, word):
        """Apply word; return the rows it moved, or None when a letter is inadmissible."""
        k = 0
        while k < len(self.stack) and k < len(word) and self.stack[k][0] == word[k]:
            k += 1
        self.pop(k)
        for letter in word[k:]:
            g, i = letter
            if self.rs.ns[g] != 0:
                return None
            moved = np.flatnonzero(self.rs.lab[:, g] == 1)
            ctx = self.rs.applied(g, i)
            ctx.__enter__()
            self.stack.append((letter, ctx, moved))
        rows = [m for _, _, m in self.stack]
        return np.unique(np.concatenate(rows)) if rows else np.zeros(0, np.int64)

    def pop(self, k=0):
        while len(self.stack) > k:
            self.stack.pop()[1].__exit__(None, None, None)


class Tails:
    """The tails (a) to (e) at a configuration where r has no straddler."""

    def __init__(self, rs, near, deadline=None):
        self.rs, self.near, self.deadline = rs, near, deadline
        self.nm = np.zeros((600, 600), bool)
        for c, grips in enumerate(near):
            self.nm[c, sorted(grips)] = True
        self.counts = dict(states=0, letters=0, leaf_tests=0)

    def own_high(self, cand):
        """The grips of cand that hold a high row of their own branch."""
        rs = self.rs
        hr = np.flatnonzero(rs.sec >= 0)
        if not len(cand) or not len(hr):
            return cand[:0]
        return cand[((rs.lab[np.ix_(hr, cand)] == 1) & (rs.b[hr][:, None] == CLS[cand][None, :])).any(0)]

    def last_letter(self, g, allowed, cache, emit):
        """Every retained item at the clean grip g, then the lowerings at targets in allowed.

        A target must be clean after the letter and hold a high row of its own
        branch (otherwise D is 0); both are read before any metric.
        """
        rs = self.rs
        S.tick(self.deadline)
        moved = np.flatnonzero(rs.lab[:, g] == 1)
        m2 = (rs.lab[moved] == 2).T @ rs.w[moved]
        rest = rs.ns - m2
        allowed = allowed.copy()
        allowed[g] = False
        split = None
        for i in range(1, 12):
            self.counts['letters'] += 1
            _, pre, _, sigma = A.retained(g, i)
            cand = np.flatnonzero(((rest + m2[pre]) == 0) & allowed)
            if not len(cand):
                continue
            if split is None:
                is_moved = np.zeros(len(rs.w), bool)
                is_moved[moved] = True
                high = rs.sec >= 0
                h_out, h_in = np.flatnonzero(high & ~is_moved), np.flatnonzero(high & is_moved)
                split = h_out, h_in, rs.lab[h_in], rs.b[h_in]
            h_out, h_in, lab_in, b_in = split
            own = CLS[cand][None, :]
            hold = ((rs.lab[np.ix_(h_out, cand)] == 1) & (rs.b[h_out][:, None] == own)).any(0)
            if len(h_in):
                hold |= ((lab_in[:, pre[cand]] == 1) & (sigma[b_in][:, None] == own)).any(0)
            for x in cand[hold].tolist():
                self.counts['leaf_tests'] += 1
                if x not in cache:
                    cache[x] = metric(rs, x)
                s, f, ds = rs.leaf(x, g, i, cache[x], moved)
                A.require(s == 0, 'leaf clean')
                emit(((g, i),), x, f, ds)

    def search(self, r, emit):
        """Call emit(tail, x, F, D vector) for every tail lowering; r must be clean."""
        rs = self.rs
        A.require(rs.ns[r] == 0, 'released')
        self.counts['states'] += 1
        cache = {}
        clean = np.flatnonzero(rs.ns == 0)
        # (a) a lowering right after the routing word
        for x in self.own_high(clean).tolist():
            self.counts['leaf_tests'] += 1
            cache[x] = metric(rs, x)
            s, f, ds = cache[x]
            emit((), x, f, ds)
        clean = clean.tolist()
        # (b) u at a clean grip other than r, then a lowering at a partner of u's grip
        for y in clean:
            if y != r:
                self.last_letter(y, self.nm[y], cache, emit)
        # (c) the releaser, then a lowering at a partner of r
        self.last_letter(r, self.nm[r], cache, emit)
        # (d) the releaser, u, then a lowering at a partner of u's grip
        for m in range(1, 12):
            with rs.applied(r, m):
                cache_m = {}
                for y in np.flatnonzero(rs.ns == 0).tolist():
                    if y == r:
                        continue
                    allowed = self.nm[y] if self.nm[y, r] else self.nm[y] & self.nm[r]
                    self.last_letter(y, allowed, cache_m, lambda t, x, f, ds, m=m: emit(((r, m), *t), x, f, ds))
        # (e) u at a clean partner of r, the releaser, then a lowering at a partner of r
        for y in clean:
            if y == r or not self.nm[y, r]:
                continue
            for i in range(1, 12):
                with rs.applied(y, i):
                    if rs.ns[r] != 0:
                        continue
                    self.last_letter(r, self.nm[r], {}, lambda t, x, f, ds, y=y, i=i: emit(((y, i), *t), x, f, ds))


def state_key(rs, rows, root):
    lab, b, sec = root
    if len(rows):
        diff = rows[(rs.lab[rows] != lab[rows]).any(1) | (rs.b[rows] != b[rows]) | (rs.sec[rows] != sec[rows])]
    else:
        diff = rows
    h = hashlib.blake2b(digest_size=16)
    for arr in (diff.astype(np.int64), rs.lab[diff], rs.b[diff], rs.sec[diff]):
        h.update(np.ascontiguousarray(arr).tobytes())
    return h.digest()


def pairs_at(state, rs):
    """Every (c, r): r blocked, c clean and holding every straddler of r."""
    lab = state.lab
    clean = np.flatnonzero(rs.ns == 0).tolist()
    out = []
    for r in np.flatnonzero(rs.ns != 0).tolist():
        held = lab[lab[:, r] == 2]
        out.extend((c, r) for c in clean if (held[:, c] == 1).all())
    return out


def search(state, nmax=2, max_helpers=2, deadline=None, pairs=None, count_only=False, progress=None, nmin=1):
    """Return (best row per target, per-pair records, counts) for nmin <= n <= nmax carrier actions."""
    A.require(1 <= nmin <= nmax <= 3 and 0 <= max_helpers <= 3, 'bounds')
    rs = S.RetainedSearch(state)
    phi0 = J.phi(state.h)
    near = X.nbr()
    digest = state.st.digest()
    root = (rs.lab.copy(), rs.b.copy(), rs.sec.copy())
    tails = Tails(rs, near, deadline)
    best, records = {}, []
    counts = dict(nmin=nmin, nmax=nmax, helpers=max_helpers, pairs=0, words=0, admissible=0, releases=0,
                  distinct_releases=0, hits=0)
    for c, r in (pairs if pairs is not None else pairs_at(state, rs)):
        S.tick(deadline)
        counts['pairs'] += 1
        an = RT.anatomy(state, state.lab, c, r, verbose=False)
        rec = dict(c=c, r=r, S_r=int((state.lab[:, r] == 2).sum()), seqs=[len(an['seqs'][n]) for n in (1, 2, 3)],
                   words=[0, 0, 0], admissible=0, releases=0, distinct=0, hits=0, best=None)
        states = {}
        walker = Walker(rs)
        try:
            for nn, word in routing_words(an, nmax, max_helpers, nmin=nmin):
                rec['words'][nn-1] += 1
                if count_only:
                    continue
                S.tick(deadline)
                rows = walker.go(word)
                if rows is None:
                    continue
                rec['admissible'] += 1
                if rs.ns[r] != 0:
                    continue
                rec['releases'] += 1
                key = state_key(rs, rows, root)
                if key in states:
                    old = states[key]
                    if (len(word), word) < (len(old[0]), old[0]):
                        old[0] = list(word)
                    continue
                found = {}

                def emit(tail, x, f, ds):
                    for i, d in zip(P.JUMBLES, ds.tolist()):
                        if f < d:
                            row = (phi0 + 2*(f-d), len(tail), [list(e) for e in tail] + [[x, i]], f, d)
                            if x not in found or row[:3] < found[x][:3]:
                                found[x] = row

                tails.search(r, emit)
                states[key] = [list(word), found]
        finally:
            walker.pop()
        rec['distinct'] = len(states)
        for word, found in states.values():
            for x, (phi, _, tail, f, d) in found.items():
                rec['hits'] += 1
                row = dict(S.raw_candidate([*word, *map(tuple, tail)], phi, 0), F=f, D=d, target=x,
                           carrier=c, released=r)
                if x not in best or S.candidate_key(row) < S.candidate_key(best[x]):
                    best[x] = row
                if rec['best'] is None or S.candidate_key(row) < S.candidate_key(rec['best']):
                    rec['best'] = row
        for k in ('words', 'admissible', 'releases', 'distinct', 'hits'):
            v = rec[k]
            counts[k if k != 'distinct' else 'distinct_releases'] += sum(v) if isinstance(v, list) else v
        records.append(rec)
        if progress:
            progress(rec)
    counts.update(tails.counts)
    A.require(state.st.digest() == digest, 'undo', 'family RR')
    rows = sorted(best.values(), key=S.candidate_key)
    return rows, records, counts


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('seed')
    ap.add_argument('out')
    ap.add_argument('--word', default='')
    ap.add_argument('--labels')
    ap.add_argument('--nmin', type=int, default=1)
    ap.add_argument('--nmax', type=int, default=2)
    ap.add_argument('--helpers', type=int, default=2)
    ap.add_argument('--pairs', default='')
    ap.add_argument('--count-only', action='store_true')
    a = ap.parse_args(argv)
    started = time.process_time()
    state = C.load(a.seed, a.word, a.labels)
    pairs = [tuple(P.grip(int(v)) for v in p.split(':')) for p in a.pairs.split(',')] if a.pairs else None
    result = C.record('rr', a.seed, state, a.word, {k: v for k, v in vars(a).items() if k not in ('seed', 'out')})

    def show(rec):
        print(f"  {rec['c']}->{rec['r']} S {rec['S_r']} seqs {rec['seqs']} words {rec['words']} admissible "
              f"{rec['admissible']} releases {rec['releases']} distinct {rec['distinct']} hits {rec['hits']}"
              + (f" best {rec['best']['terminal_phi']} {rec['best']['word']}" if rec['best'] else '')
              + f" (cpu {round(time.process_time()-started, 1)})", flush=True)

    rows, records, counts = search(state, a.nmax, a.helpers, None, pairs, a.count_only, show, a.nmin)
    result.update(counts=counts, pairs=records, best_per_target=rows, complete=True,
                  cpu_s=round(time.process_time()-started, 3))
    C.atomic_json(a.out, result)
    print(a.seed, 'phi', result['phi'], counts, 'best', rows[0]['terminal_phi'] if rows else None,
          rows[0]['word'] if rows else None, 'cpu', result['cpu_s'], flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
