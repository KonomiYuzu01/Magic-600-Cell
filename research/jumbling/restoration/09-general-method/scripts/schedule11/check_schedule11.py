"""Acceptance check for the round 11 schedule of the retained cascade (SENIOR-REVIEW.md section 16).

usage (from the repository root): python3 -I -B research/jumbling/restoration/09-general-method/scripts/schedule11/check_schedule11.py

It checks the module `sched11.py` and the runner `restore_schedule11.py` of this directory:
1. The per-orbit worst cases of T11-05 (its reproduction program, per orbit, in twists) sum to 20,866,844.
2. The acceptance inputs that T11-05 asks for: frames wrong with every position in place, closing three-cycles,
   paired transpositions, and the non-abelian cleanup on O06 and O34. Each input is solved orbit by orbit in the
   cascade order with `Schedule11.solve_orbit`. The straight-line program is then evaluated again here, with the
   rules of the stage 03 checker (`checker/verify_restore.py`, copied, not imported; a node that it cannot evaluate,
   such as an empty product or a letter that is not an integer, fails here too), and applied to a copy of the
   input: after each orbit, that orbit and every earlier one must be solved; at the end the state must be the
   identity on all 259,800 stickers; and each orbit's length must be at most its T11-05 bound.
3. The runner on uniform:1 and uniform:2: the summary and the solutions hold exactly these two inputs, the stored
   start arrays hash to the committed digests, and both runs are restored. The written straight-line program is
   evaluated here and cut into the per-orbit slices that `factors_per_orbit` gives in the cascade order; the slices
   are applied in turn to the start array: after each one, that orbit and every earlier one must be solved, and the
   slice's length, recomputed here, must equal the reported one and be at most the orbit's bound. Each total is at
   most 20,866,844 twists. Then the unchanged stage 03 checker must PASS on the same output, for exactly these
   two inputs.
Exit status 0 only if every check passes. Importing the module runs only part 1 and the set-up (the mutation test
`mutants_check11.py` uses that). Research only: exact synthetic geometry; no performance claim.
"""
import gzip
import hashlib
import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import numpy as np

T0 = time.time()
ROOT = Path.cwd()
HERE = Path(__file__).resolve().parent
GM = HERE.parents[1]
S03 = GM.parent / '03-retained-group-and-lattice-restorer'
REALIGN = GM / 'data' / 'realign.json.gz'
sys.path.insert(0, str(S03 / 'scripts'))
sys.path.insert(0, str(HERE))
from rlib import Retained, NS             # noqa: E402
from kit import SLP, OrbitKit             # noqa: E402
from sched11 import Schedule11            # noqa: E402

TOTAL = 20866844
HG = {0: 'C2', 3: 'C2', 8: 'C2', 11: 'C2', 22: 'C2', 25: 'C2', 32: 'C2', 17: 'C5', 28: 'C5', 6: 'D5', 34: 'A5'}
FAILS = []


def check(cond, msg):
    if not cond:
        FAILS.append(msg)
        print('FAIL', msg, flush=True)
    return cond


# ---- 1. per-orbit bounds (T11-05 reproduction program, per orbit, twists) ---------------------------
def bounds11():
    T = json.load(gzip.open(ROOT / 'assets/execution_trees.json.gz', 'rt'))
    J = json.load(gzip.open(REALIGN, 'rt'))
    S = {r['orbit']: r['s'] for r in J['orbits']}
    e = [18, 11, 9, 9, 8, 8, 8, 7, 7, 7] + [6] * 18 + [5, 5] + [6] * 4 + [4]
    a = [18, 11, 9, 9, 7, 8, 8, 7, 7, 7, 6, 6, 6, 6, 6, 6, 6, 6, 5, 6, 6, 6, 5, 5, 5, 5, 6, 5, 5, 5, 6, 5, 5, 5, 5]
    b = {0: 19, 3: 9, 6: 8, 8: 7, 11: 6, 17: 6, 22: 6, 25: 5, 28: 5, 32: 5, 34: 5}
    f = {0: 18, 3: 9, 6: 8, 8: 7, 11: 6, 17: 6, 22: 5, 25: 5, 28: 5, 32: 5, 34: 5}
    H = {0: 2, 3: 2, 6: 10, 8: 2, 11: 2, 17: 5, 22: 2, 25: 2, 28: 5, 32: 2, 34: 60}
    h = {0: 5, 3: 3, 6: 4, 8: 3, 11: 3, 17: 2, 22: 3, 25: 2, 28: 2, 32: 2, 34: 3}
    tau = {0: 34, 3: 15, 6: 13, 8: 9, 11: 7, 17: 9, 22: 9, 25: 7, 28: 8, 32: 7, 34: 8}
    out = {}
    for o, t in enumerate(T):
        n = len(set(t['positions'])) + 2
        D = e[o] + max(t['depth'])
        K = a[o] + D
        z = S[o]
        if o not in H:
            out[o] = n // 2 * (z + 2 * K)
            continue
        P = 2 * z + 2 * h[o] + 2 * f[o]
        Q = 2 * z + 2 * h[o] + 2 * (b[o] + f[o]) if H[o] == 2 else 2 * z + 4 * h[o] + 4 * tau[o] + 2 * K
        W = z + 4 * h[o] + 4 * tau[o] + 2 * D
        C = 4 * z + 4 * h[o] + (4 * z + (16 if o == 6 else 10) if o in (6, 34) else 0)
        paired = Q < 2 * P

        def fc(j):
            return j // 2 * Q + (j % 2) * P if paired else j * P
        worst = fc(n - 3)
        for m in range(3, n + 1):
            k = m // 2
            k -= (k - m) % 2
            for i in (0, 1):
                if i == 0 and m == n:
                    continue
                worst = max(worst, (m + k - 2 * i) // 2 * W + fc(n - m - (1 - i)))
        out[o] = worst + C
    return out


BOUND = bounds11()
check(sum(BOUND.values()) == TOTAL, 'per-orbit bounds sum to %d, not %d' % (sum(BOUND.values()), TOTAL))
print('bounds: sum %d; largest %s' % (sum(BOUND.values()),
      sorted(BOUND.items(), key=lambda x: -x[1])[:3]), flush=True)

# ---- labels, frame groups and invariants (as restore_realigned.py lines 61 to 146) ------------------
R = Retained('.')
RJ = json.load(gzip.open(REALIGN, 'rt'))
_slp = SLP(R)
coll, kits = {}, {}
for o in range(35):
    sn = _slp.word(R.trees[o]['seed'])
    coll[o] = sorted(set(int(x) for x in R.so[_slp.net[sn][0]]) - {o})
    w = [int(m) for m in R.trees[o]['relocation']]
    reloc = _slp._add({'op': 'word', 'w': w}, (np.zeros(0, np.int64), np.zeros(0, np.int64)), len(w))
    _slp.word_of[reloc] = w
    kits[o] = OrbitKit(R, _slp, o, _slp.conj(sn, reloc), allowed=set(coll[o]), compact_paths=True)


def frame_group(o):
    kit = kits[o]
    k = kit.k
    ident = tuple(range(k))
    if o not in HG:
        return [ident], None, 1
    kit._pairs()
    vals = list(kit.pair_by_value)
    H = set(vals)
    fr = list(vals)
    while fr:
        nx = []
        for x in fr:
            for g in vals:
                y = tuple(g[x[i]] for i in range(k))
                if y not in H:
                    H.add(y)
                    nx.append(y)
        fr = nx

    def order_of(x):
        y, n = x, 1
        while y != ident:
            y, n = tuple(x[y[i]] for i in range(k)), n + 1
        return n
    if HG[o] == 'C2':
        return sorted(H), {g: int(g != ident) for g in H}, 2
    if HG[o] == 'D5':
        return sorted(H), {g: int(order_of(g) == 2) for g in H}, 2
    if HG[o] == 'C5':
        r0 = min(g for g in H if order_of(g) == 5)
        m, x = {}, ident
        for i in range(5):
            m[x] = i
            x = tuple(r0[x[j]] for j in range(k))
        return sorted(H), m, 5
    return sorted(H), {g: 0 for g in H}, 1


FG = {o: frame_group(o) for o in range(35)}
del _slp
print('frame groups %s (%.1fs)' % ({o: len(FG[o][0]) for o in HG}, time.time() - T0), flush=True)


class Config:
    """A retained configuration under construction: position X holds the piece of home Y with frame v."""

    def __init__(self):
        self.L = np.arange(NS, dtype=np.int64)
        self.held = {o: {} for o in range(35)}

    def put(self, o, X, Y, v):
        kit = kits[o]
        assert X not in self.held[o]
        self.held[o][X] = (Y, v)
        for t in range(kit.k):
            self.L[kit.lab[X][v[t]]] = kit.lab[Y][t]

    def finish(self):
        """Fill the rest with solved pieces and check that the configuration lies in G."""
        for o in range(35):
            kit = kits[o]
            ident = tuple(range(kit.k))
            for X in kit.lab:
                if X not in self.held[o]:
                    self.put(o, X, X, ident)
            perm = {X: Y for X, (Y, _) in self.held[o].items()}
            assert sorted(perm.values()) == sorted(perm)
            seen, par = set(), 0
            for x in perm:
                n = 0
                while x not in seen:
                    seen.add(x)
                    x = perm[x]
                    n += 1
                par ^= (max(n, 1) - 1) & 1
            assert par == 0, 'odd position permutation on orbit %d' % o
            H, tab, mod = FG[o]
            assert all(v in H for _, v in self.held[o].values())
            if tab is not None:
                assert sum(tab[v] for _, v in self.held[o].values()) % mod == 0, 'frame invariant on orbit %d' % o
        assert np.array_equal(np.sort(self.L), np.arange(NS))
        return self.L


def others(o):
    kit = kits[o]
    return [p for p in sorted(kit.lab) if p not in (kit.A, kit.B, kit.C)]


def frames_for(o, rng, count):
    """count random frames whose invariant is zero (the last one fixes it)."""
    H, tab, mod = FG[o]
    fr = [H[i] for i in rng.integers(0, len(H), count)]
    if tab is not None:
        need = (-sum(tab[v] for v in fr[:-1])) % mod
        fr[-1] = next(v for v in H if tab[v] == need)
    return fr


def input_frames_in_place():
    """T10-03 and T11-05: every position in place, frames wrong on almost every position."""
    c = Config()
    rng = np.random.default_rng(11)
    for o in HG:
        H, tab, mod = FG[o]
        ident = tuple(range(kits[o].k))
        ps = others(o)
        if HG[o] in ('C2', 'D5'):
            g = min(v for v in H if tab[v] == 1)
            for X in ps[:-1]:                 # N - 4 positions: even
                c.put(o, X, X, g)
        elif HG[o] == 'C5':
            g = next(v for v in H if tab[v] == 1)
            n = len(ps) - len(ps) % 5
            for X in ps[:n]:
                c.put(o, X, X, g)
        else:
            nonid = [v for v in H if v != ident]
            for X in ps:
                c.put(o, X, X, nonid[int(rng.integers(len(nonid)))])
    return c.finish()


def input_three_cycles():
    """On every orbit: a closing three-cycle through A (A holds X, X holds Y, Y holds A) and a three-cycle away from
    the buffers, with random frames on the six moved positions."""
    c = Config()
    rng = np.random.default_rng(12)
    for o in range(35):
        kit = kits[o]
        ps = others(o)
        X, Y, P1, P2, P3 = ps[0], ps[len(ps) // 2], ps[1], ps[2], ps[-1]
        moves = [(kit.A, X), (X, Y), (Y, kit.A), (P1, P2), (P2, P3), (P3, P1)]
        for (pos, home), v in zip(moves, frames_for(o, rng, len(moves))):
            c.put(o, pos, home, v)
    return c.finish()


def input_transpositions():
    """On every orbit: two disjoint transpositions away from the buffers, and a transposition through A with another
    one, with random frames on the eight moved positions."""
    c = Config()
    rng = np.random.default_rng(13)
    for o in range(35):
        kit = kits[o]
        ps = others(o)
        P, Q, R_, S_, U, V, W = ps[3], ps[-2], ps[4], ps[5], ps[6], ps[-3], ps[7]
        moves = [(P, Q), (Q, P), (R_, S_), (S_, R_), (kit.A, U), (U, kit.A), (V, W), (W, V)]
        for (pos, home), v in zip(moves, frames_for(o, rng, len(moves))):
            c.put(o, pos, home, v)
    return c.finish()


def input_cleanup_c():
    """O06 and O34: every piece solved except the frame at C, a nonidentity value of the commutator subgroup."""
    c = Config()
    for o in (6, 34):
        H, tab, _ = FG[o]
        kit = kits[o]
        ident = tuple(range(kit.k))
        v = max(x for x in H if x != ident and tab[x] == 0)
        c.put(o, kit.C, kit.C, v)
    return c.finish()


def input_cleanup_abc():
    """O06 and O34: positions in place; random frames at A, B and C only."""
    c = Config()
    rng = np.random.default_rng(14)
    for o in (6, 34):
        kit = kits[o]
        for X, v in zip((kit.A, kit.B, kit.C), frames_for(o, rng, 3)):
            c.put(o, X, X, v)
    return c.finish()


# ---- independent evaluation of a straight-line program (rules of checker/verify_restore.py) -------
TREES = R.trees


def tree_path(o, n):
    t = TREES[o]
    out = []
    while t['parent'][n] >= 0:
        out.append(int(t['move'][n]))
        n = t['parent'][n]
    return out[::-1]


_MV = {}


def move(m):
    assert isinstance(m, int) and 0 <= abs(m) - 1 < 1200, m      # on every call: stage 03 checks uncached letters
    r = _MV.get(m)
    if r is None:
        i = abs(m) - 1
        s, d = R.src[R.off[i]:R.off[i + 1]], R.dst[R.off[i]:R.off[i + 1]]
        if m < 0:
            s, d = d, s
        ix = np.argsort(s)
        r = (s[ix], d[ix])
        _MV[m] = r
    return r


def follow(x, w):
    x = np.array(x, np.int64, copy=True)
    for m in w:
        s, d = move(m)
        ix = np.minimum(np.searchsorted(s, x), len(s) - 1)
        hit = s[ix] == x
        x[hit] = d[ix[hit]]
    return x


def srt(s, d):
    ix = np.argsort(s)
    return s[ix], d[ix]


def evaluate(nodes, ids):
    """nets and lengths of the nodes `ids` (ascending), each node a dict as in the solutions.json format, with
    references to earlier ids. A net exists only for conj and prod nodes and for a word used as a conjugated element,
    exactly as in the stage 03 checker."""
    net, word, length = {}, {}, {}
    L = np.arange(NS, dtype=np.int64)
    for i in ids:
        nd = nodes[i]
        op = nd['op']
        if op in ('word', 'path'):
            # NumPy integers of in-process nodes become ints; anything else reaches move() unchanged, as in stage 03
            w = ([int(m) if isinstance(m, np.integer) else m for m in nd['w']] if op == 'word'
                 else tree_path(nd['orbit'], nd['node']))
            word[i] = w
            length[i] = len(w)
        elif op == 'conj':
            x, u, us = nd['x'], nd['u'], nd['us']
            if x not in net:
                for m in word[x]:
                    s, d = move(m)
                    L[d] = L[s]
                y = np.flatnonzero(L != np.arange(NS))
                net[x] = srt(L[y].copy(), y)
                L[:] = np.arange(NS)
            s, d = net[x]
            if u in word:
                w = word[u] if us > 0 else [-m for m in reversed(word[u])]
                net[i] = srt(follow(s, w), follow(d, w))
            else:
                us_s, us_d = net[u] if us > 0 else srt(net[u][1], net[u][0])

                def ap(a):
                    a = a.copy()
                    ix = np.minimum(np.searchsorted(us_s, a), max(len(us_s) - 1, 0))
                    hit = us_s[ix] == a if len(us_s) else np.zeros(len(a), bool)
                    a[hit] = us_d[ix[hit]]
                    return a
                net[i] = srt(ap(s), ap(d))
            length[i] = length[x] + 2 * length[u]
        elif op == 'prod':
            if not nd['f']:
                raise ValueError('node %d: empty product (the stage 03 checker cannot evaluate it)' % i)
            touched = []
            for j, sg in nd['f']:
                s, d = net[j] if sg > 0 else srt(net[j][1], net[j][0])
                L[d] = L[s]
                touched.append(d)
            t = np.unique(np.concatenate(touched))
            lt = L[t]
            mv = lt != t
            net[i] = srt(lt[mv].copy(), t[mv])
            L[t] = t
            length[i] = sum(length[j] for j, _ in nd['f'])
        else:
            raise ValueError(op)
    return net, length


def closure(nodes, roots):
    used, stack = set(), list(roots)
    while stack:
        i = stack.pop()
        if i in used:
            continue
        used.add(i)
        nd = nodes[i]
        if nd['op'] == 'conj':
            stack += [nd['x'], nd['u']]
        elif nd['op'] == 'prod':
            stack += [f[0] for f in nd['f']]
    return sorted(used)


def apply_factors(L, net, fs):
    for j, sg in fs:
        s, d = net[j] if sg > 0 else srt(net[j][1], net[j][0])
        L[d] = L[s]


# ---- 2. acceptance inputs -------------------------------------------------------------------------
SO = R.so


def solved_orbits(L, orbits):
    idx = np.flatnonzero(np.isin(SO, list(orbits)))
    return bool(np.array_equal(L[idx], idx))


def run_input(name, L0):
    t0 = time.time()
    slp = SLP(R)
    sch = Schedule11(R, slp, RJ)
    if not check(list(sch.cascade) == RJ['cascade_order'], '%s: cascade order differs' % name):
        return
    L = L0.copy()
    per = {}
    for o in sch.cascade:
        per[o] = [(int(i), int(sg)) for i, sg in sch.solve_orbit(o, L)]
    t1 = time.time()
    nodes = slp.nodes
    try:
        net, length = evaluate(nodes, closure(nodes, [i for fs in per.values() for i, _ in fs]))
    except Exception as exc:                      # a node the stage 03 checker could not evaluate
        check(False, '%s: independent evaluation failed: %r' % (name, exc))
        return
    L2 = L0.copy()
    done, worst = [], (0.0, None)
    for o in sch.cascade:
        apply_factors(L2, net, per[o])
        done.append(o)
        if not check(solved_orbits(L2, done), '%s: after orbit %d an orbit up to it is not solved' % (name, o)):
            return
        n_ind = sum(length[i] for i, _ in per[o])
        n_own = sum(slp.length[i] for i, _ in per[o])
        check(n_ind == n_own, '%s: orbit %d length %d, recomputed %d' % (name, o, n_own, n_ind))
        check(n_ind <= BOUND[o], '%s: orbit %d length %d above its bound %d' % (name, o, n_ind, BOUND[o]))
        worst = max(worst, (n_ind / BOUND[o], o))
    check(np.array_equal(L2, np.arange(NS)), '%s: not the identity' % name)
    total = sum(length[i] for fs in per.values() for i, _ in fs)
    check(total <= TOTAL, '%s: total %d above %d' % (name, total, TOTAL))
    print('%-16s total %10d  factors %7d  largest length/bound %.3f (orbit %s)  solve %.0fs, check %.0fs'
          % (name, total, sum(len(fs) for fs in per.values()), worst[0], worst[1], t1 - t0, time.time() - t1),
          flush=True)


INPUTS = (('frames-in-place', input_frames_in_place), ('three-cycles', input_three_cycles),
          ('transpositions', input_transpositions), ('cleanup-C', input_cleanup_c),
          ('cleanup-ABC', input_cleanup_abc))

# ---- 3. the runner on the committed synthetic inputs ------------------------------------------------
COMMITTED = json.loads((S03 / 'restore' / 'restore-summary.json').read_text())
DIGEST = {r['input']: r['input_labels_sha256'] for r in COMMITTED['runs'] if 'input_labels_sha256' in r}
SPECS = ['uniform:1', 'uniform:2']
TAGS = [s.replace(':', '_') for s in SPECS]


def check_run(run, fs, net, length, out):
    """One runner result, replayed orbit by orbit with the slices that factors_per_orbit gives."""
    spec = run['input']
    L = np.load(out / ('start-labels-%s.npy' % spec.replace(':', '_')))
    check(hashlib.sha256(L.astype('<i8').tobytes()).hexdigest() == DIGEST[spec],
          '%s: start array is not the committed input' % spec)
    check(run['status'] == 'restored', '%s: status %s' % (spec, run['status']))
    cascade = RJ['cascade_order']
    fpo = {int(o): n for o, n in run['factors_per_orbit'].items()}
    lpo = {int(o): n for o, n in run['length_per_orbit'].items()}
    if not check(sorted(fpo) == sorted(lpo) == sorted(cascade)
                 and all(type(n) is int and n >= 0 for n in fpo.values())
                 and sum(fpo.values()) == len(fs) == run['factors'],
                 '%s: factors_per_orbit does not cut the solution into one slice per orbit' % spec):
        return
    pos, done, total, worst = 0, [], 0, (0.0, None)
    for o in cascade:
        part = fs[pos:pos + fpo[o]]
        pos += fpo[o]
        apply_factors(L, net, part)
        done.append(o)
        if not check(solved_orbits(L, done), '%s: after orbit %d an orbit up to it is not solved' % (spec, o)):
            return
        n = sum(length[j] for j, _ in part)
        total += n
        check(n == lpo[o], '%s: orbit %d reported length %d, recomputed from its slice %d' % (spec, o, lpo[o], n))
        check(n <= BOUND[o], '%s: orbit %d length %d above its bound %d' % (spec, o, n, BOUND[o]))
        worst = max(worst, (n / BOUND[o], o))
    check(np.array_equal(L, np.arange(NS)), '%s: the written program does not restore the input' % spec)
    check(total == run['retained_length'], '%s: length %d, recomputed %d' % (spec, run['retained_length'], total))
    check(total <= TOTAL, '%s: total %d above %d' % (spec, total, TOTAL))
    print('%-16s total %10d  factors %7d  largest length/bound %.3f (orbit %s)'
          % (spec, total, len(fs), worst[0], worst[1]), flush=True)


def check_runner_output(out):
    summary = json.loads((out / 'restore-summary.json').read_text())
    sol = json.loads((out / 'solutions.json').read_text())
    check(sol.get('format') == 'lattice-restoration-v1', 'solutions format')
    check(summary.get('cascade_order') == RJ['cascade_order'], 'runner: cascade order differs')
    runs = summary.get('runs', [])
    got = [r.get('input') for r in runs]
    ok = check(got == SPECS, 'runner: runs for %s, expected exactly %s' % (got, SPECS))
    got = sorted(sol.get('solutions', {}))
    ok &= check(got == TAGS, 'runner: solutions for %s, expected exactly %s' % (got, TAGS))
    if not ok:
        return
    try:
        net, length = evaluate(sol['nodes'], range(len(sol['nodes'])))
    except Exception as exc:                      # a node the stage 03 checker could not evaluate
        check(False, 'runner: independent evaluation failed: %r' % (exc,))
        return
    for run in runs:
        try:
            check_run(run, sol['solutions'][run['input'].replace(':', '_')], net, length, out)
        except Exception as exc:                  # a malformed record or factor
            check(False, '%s: replay failed: %r' % (run['input'], exc))


def runner_check():
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / 'run'
        t0 = time.time()
        p = subprocess.run([sys.executable, '-I', '-B', str(HERE / 'restore_schedule11.py'), str(REALIGN), str(out)]
                           + SPECS, cwd=ROOT, capture_output=True, text=True, timeout=1200)
        print(p.stdout[-2000:], p.stderr[-2000:], flush=True)
        if check(p.returncode == 0, 'runner exit status %d' % p.returncode):
            check_runner_output(out)
            # the unchanged stage 03 checker on the same output; it imports the simulator of research/jumbling
            rep = Path(tmp) / 'verify-report.json'
            q = subprocess.run([sys.executable, '-I', str(S03 / 'checker' / 'verify_restore.py'), str(ROOT),
                                str(GM.parents[1]), str(S03 / 'restore' / 'library-final.json'), str(out), str(rep)],
                               cwd=ROOT, capture_output=True, text=True, timeout=1200)
            print(q.stdout[-1500:], q.stderr[-1500:], flush=True)
            vr = json.loads(rep.read_text()) if q.returncode == 0 and rep.is_file() else {}
            vruns = vr.get('runs', [])
            check(vr.get('pass') is True and [r.get('input') for r in vruns] == SPECS
                  and all(r.get('synthetic_input_valid') and r.get('restored_to_identity')
                          and r.get('length_matches_restorer') for r in vruns),
                  'stage 03 checker: no PASS for exactly %s (exit status %d)' % (SPECS, q.returncode))
        print('runner and its checks %.0fs' % (time.time() - t0), flush=True)


if __name__ == '__main__':
    for name, build in INPUTS:
        run_input(name, build())
    runner_check()
    print('%s in %.0fs' % ('PASS' if not FAILS else 'FAIL (%d)' % len(FAILS), time.time() - T0))
    sys.exit(1 if FAILS else 0)
