"""Stage 17 acceptance gate: every H0--H4 assertion is mandatory.

One thread, sequential searches, external TemporaryDirectory scratch only.
A wall alarm fails the gate at 3600 seconds. The measurements concern exact
synthetic geometry, not native-host or performance evidence.

Run: python3 -I -B research/jumbling/restoration/17-routing-in-driver/scripts/test_rr.py
"""
import os
from pathlib import Path
import random
import resource
import signal
import sys
import tempfile
import time

sys.dont_write_bytecode = True
for _var in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_var] = '1'


def expired(signum, frame):
    raise AssertionError('acceptance timeout: 3600 wall seconds')


signal.signal(signal.SIGALRM, expired)
signal.alarm(3600)
SCRIPTS = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS))
import rr as RR  # noqa: E402
import pxrr as PXRR  # noqa: E402

C, S, P, A, J, np = RR.C, RR.S, RR.P, RR.A, RR.J, RR.np
RT = RR.RT
ASSERTIONS = 0
D7 = [(561, 1), (598, 4), (233, 6), (536, 11), (370, 15)]  # O-SJ-D7 as certified in stage 16


def expect(ok, detail):
    global ASSERTIONS
    ASSERTIONS += 1
    if not ok:
        raise AssertionError(detail)


def cpu_total():
    children = resource.getrusage(resource.RUSAGE_CHILDREN)
    return time.process_time() + children.ru_utime + children.ru_stime


class Row:
    def __init__(self, name):
        self.name = name

    def __enter__(self):
        self.cpu = cpu_total()

    def __exit__(self, kind, value, tb):
        print(f'{self.name}: {"FAIL" if kind else "PASS"} cpu_s={cpu_total()-self.cpu:.3f}', flush=True)


def reduce_word(word, c):
    """Merge consecutive carrier letters at c and drop identities."""
    out = []
    for g, i in word:
        if g == c and out and out[-1][0] == c:
            k = C.prod(c, i, out[-1][1])
            out.pop()
            if k:
                out.append((c, k))
        else:
            out.append((g, i))
    return out


def brute_tails(rs, r, phi0):
    """Best (terminal phi, length) per target over the whole tail language, without reductions.

    Every word of at most two retained letters at clean grips (each admissible
    when it acts) with at most one letter at a grip other than r, then every
    lowering at a grip that is clean after it, read with the metric of H0.
    """
    best = {}

    def lower(prefix):
        for x in np.flatnonzero(rs.ns == 0).tolist():
            if prefix and x == prefix[-1][0]:
                continue
            s, f, ds = RR.metric(rs, x)
            for d in ds.tolist():
                if f < d:
                    key = (phi0 + 2*(f-d), len(prefix) + 1)
                    best[x] = min(best.get(x, key), key)

    lower([])
    for g1 in np.flatnonzero(rs.ns == 0).tolist():
        for i1 in range(1, 12):
            with rs.applied(g1, i1):
                lower([(g1, i1)])
                for g2 in np.flatnonzero(rs.ns == 0).tolist():
                    if g2 == g1 or r not in (g1, g2):
                        continue
                    for i2 in range(1, 12):
                        with rs.applied(g2, i2):
                            lower([(g1, i1), (g2, i2)])
    return best


def tails_best(rs, r, phi0):
    found = {}

    def emit(tail, x, f, ds):
        for d in ds.tolist():
            if f < d:
                key = (phi0 + 2*(f-d), len(tail) + 1)
                found[x] = min(found.get(x, key), key)

    RR.Tails(rs, RR.X.nbr()).search(r, emit)
    return found


def main():
    started = time.time()
    with tempfile.TemporaryDirectory(prefix='s17-gate-') as tmp:
        tmp = Path(tmp)
        state = C.load(7306, save_labels=tmp/'jam7306.npz')
        rs = S.RetainedSearch(state)
        phi0 = J.phi(state.h)
        expect(phi0 == 1490, 'the 7306 jam has phi 1490')

        with Row('H0 metric'):
            rng = random.Random(7306)
            reads = 0
            for _ in range(6):
                g, i = rng.choice(np.flatnonzero(rs.ns == 0).tolist()), rng.randrange(1, 12)
                with rs.applied(g, i):
                    for x in rng.sample(range(600), 40) + [g]:
                        a, b = rs.metric(x), RR.metric(rs, x)
                        expect(a[0] == b[0] and a[1] == b[1] and np.array_equal(a[2], b[2]),
                               f'metric differs at {x} after {g}:{i}')
                        reads += 1
            expect(reads == 246, 'H0 read count')

        with Row('H1 canonical words 7306 598->536'):
            an = RT.anatomy(state, state.lab, 598, 536, verbose=False)
            canonical = {tuple(w) for _, w in RR.routing_words(an, 2, 1)}
            every = [w for _, w in RR.routing_words(an, 2, 1, canonical=False)]
            expect(len(canonical) < len(every), 'H1 the reduction removes words')
            root = (rs.lab.copy(), rs.b.copy(), rs.sec.copy())
            walker = RR.Walker(rs)
            checked = 0
            try:
                for w in every:
                    red = reduce_word(w, 598)
                    if tuple(w) in canonical:
                        expect(red == list(w), f'H1 canonical word {w} is reduced')
                        continue
                    expect(len(red) < len(w), f'H1 non-canonical word {w} reduces')
                    expect(tuple(red) in canonical or all(g != 598 for g, _ in red),
                           f'H1 reduction of {w} is enumerated or has no carrier letter')
                    rows = walker.go(w)
                    if rows is None:
                        continue
                    key = RR.state_key(rs, rows, root)
                    rows2 = walker.go(red)
                    expect(rows2 is not None, f'H1 reduction of {w} is admissible')
                    expect(RR.state_key(rs, np.union1d(rows, rows2), root) == key,
                           f'H1 reduction of {w} reaches the same configuration')
                    checked += 1
            finally:
                walker.pop()
            expect(checked > 0, 'H1 checked non-canonical words')
            print(f'  {len(every)} words, {len(canonical)} canonical, {checked} reductions compared', flush=True)

        with Row('H2 tails against brute force 7306'):
            walker = RR.Walker(rs)
            try:
                expect(walker.go(D7[:2]) is not None and rs.ns[536] == 0, 'H2 the routing word releases 536')
                brute, found = brute_tails(rs, 536, phi0), tails_best(rs, 536, phi0)
                expect(brute and found == brute, f'H2 tails {found} brute force {brute}')
                # The review's configuration (RR-001): after D7 without its lowering and 240:3, the
                # routing word 240:9 releases 90, and the lowering 370:15 follows at once (form a).
                pre = [D7[0], D7[1], D7[3], D7[2], (240, 3)]
                expect(walker.go(pre) is not None and rs.ns[90] > 0, 'H2b 90 is blocked')
                expect(walker.go(pre + [(240, 9)]) is not None and rs.ns[90] == 0, 'H2b 240:9 releases 90')
                brute, found = brute_tails(rs, 90, phi0), tails_best(rs, 90, phi0)
                expect(found.get(370) == (1268, 1), f'H2b the lowering at 370 right after the release: {found}')
                expect(found == brute, f'H2b tails {found} brute force {brute}')
            finally:
                walker.pop()

        with Row('H3 RR finds O-SJ-D7 at 7306'):
            rows, pairs, counts = RR.search(state, 1, 2, pairs=[(598, 536)])
            expect(len(pairs) == 1 and pairs[0]['releases'] == 4 and counts['words'] == 22,
                   f'H3 counts {counts}')
            best = rows[0]
            expect(best['terminal_phi'] == 1268 and best['word'] == [list(e) for e in [D7[0], D7[1], D7[3], D7[2],
                                                                                       D7[4]]],
                   f'H3 best {best}')
            claims = A.execute(state, best['word'], 20)
            expect(claims['phi'][-1] == 1268 and claims['excursion'] == 0, f'H3 certified {claims}')
            expect(state.st.digest() != C.jam(7306)['end_digest'] and J.phi(state.h) == 1268,
                   'H3 execute applied the macro')

        with Row('H4 pxrr refuses an existing run'):
            out = tmp/'run'
            out.mkdir()
            (out/'pxrr.json').write_text('{}')
            try:
                PXRR.run('not-a-seed', out, 1)
                expect(False, 'H4 must refuse')
            except Exception as exc:  # noqa: BLE001
                expect(getattr(exc, 'test', None) == 'run exists', f'H4 refusal {exc!r}')
    print(f'all {ASSERTIONS} assertions passed, wall {time.time()-started:.0f} s', flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
