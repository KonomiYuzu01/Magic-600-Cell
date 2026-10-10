"""J3-R phase U on deep inputs (restoration.md, section 3, "Deep inputs"): read the order of the twists
from their seams, and repair a misread.

The rules read only the configuration: poses, and through them domains, seams and admissibility.
They never read a journal. On top of restore.py's close, lock, improve and shift, each step applies
the first rule that finds something, in this order:
- seam (U1a, face match): undo a cap c that some single twist aligns on every misaligned inside cut
  cell (N_c drops by at least 2 (In_c - A_c) > 0), the largest drop first;
- cover (U1b): undo a cap that no misaligned cap covers and that covers some misaligned cap, the
  largest drop first. Cap d covers c when every face piece of c's cut inside d's half-space (at
  least ten on each side) sits on an aligned cut cell;
- close, lock, improve, shift: restore.py (U2, U4, U5, U5');
- conj (U5''): (c, x)(d, y)(c, z) with x any non-identity twist, y the best single twist of a
  misaligned cap d meeting c, z the best single twist of c; kept if N drops (M2 sum);
- reopen (U7): the inverse of one of the solver's own earlier twists, most recent first, then up to
  six single-twist steps (seam, cover, close, improve), the first on another cap; kept if N ends
  lower. The solver's own moves are the solver's record, not the scramble's journal.
Every kept step lowers N (M2), so a run cannot cycle. It ends on the lattice (N = 0, M3) or 'stuck'.

Run from the repository root:
    python research/jumbling/theory/seams.py fixture <menu> <t> [--start <moves.json>] [--order <r1,r2,...>]
    python research/jumbling/theory/seams.py random <menu> <k> <seeds> [mix]
    python research/jumbling/theory/seams.py batch [--jobs N] <job> ...
    python research/jumbling/theory/seams.py facecheck <menu> <t1,t2,...>
A job is fixture:<menu>:<t> or random:<menu>:<k>:<seed>[:<mix>]; batch runs the jobs in parallel
processes (default: the number of CPUs) and writes one file per job. Results go to
results/seams-<job>.json; step logs go to stderr. --start applies the solver moves of a
results file (its "moves") first, to continue a run from where it stopped.
facecheck labels, at the true scramble states X_t of a W-J fixture, which caps' latest records are
top records (undoing it gives the configuration of the scramble without it); the journal is used
for these labels only. It reports the face match's true and false positives and misses.

Evidence kind: source and exact synthetic geometry (piece positions for the cover test are float
centroids of exact poses). Timings are cloud timings of research code, not performance evidence.
"""
import json
import os
import sys
import time
from functools import partial
from multiprocessing import Pool
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))

import restore  # noqa: E402
import rim  # noqa: E402
import sim  # noqa: E402
import wj  # noqa: E402
from exact import transpose  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

ctx = restore.ctx
data = ctx.data
I4 = restore.I4
NEAR = restore.NEAR
NF = restore._NF
KAPPA = 121 / 125 * float(data.NN)           # cut depth 121/125, in the units of the cap normals
CACHE = HERE.parents[2] / 'work' / 'jumbling-cache' / 'home_centroids.npy'
_HOME = None


def home_centroids():
    """Float centroid of every piece at home (cached under work/, which Git ignores)."""
    global _HOME
    if _HOME is None:
        if CACHE.exists():
            _HOME = np.load(CACHE)
        else:
            _HOME = np.array([[wj.to_float(v) for v in wj.centroid(ctx, p)] for p in range(rim.NP)])
            CACHE.parent.mkdir(parents=True, exist_ok=True)
            np.save(CACHE, _HOME)
    return _HOME


def positions(st):
    """Float centroid of every piece in configuration st."""
    home = home_centroids()
    P = np.zeros((rim.NP, 4))
    for pid in np.unique(st.pose_id).tolist():
        m = st.pose_id == pid
        Mf = np.array([[wj.to_float(v) for v in row] for row in st._pose_matrix(pid)])
        P[m] = home[m] @ Mf.T
    return P


class SeamRestorer(restore.Restorer):
    ORDER = ('seam', 'cover', 'close', 'lock', 'improve', 'shift', 'conj', 'reopen')

    def __init__(self, st, M, order=ORDER):
        super().__init__(st, M, order=order)
        self._bal = {}

    def _touch(self, c):
        super()._touch(c)
        for f in NEAR[int(c)]:
            self._bal.pop(int(f), None)

    # ------------------------------------------------------------------ seam reading
    def balance(self, c):
        """(In_c, Out_c, A_c): cut cells covered inside, outside, and on both sides by one domain."""
        if c not in self._bal:
            ins = self.inside(c)
            if ins is None:
                self._bal[c] = None
            else:
                mask = np.zeros(rim.NP, bool)
                mask[ins] = True
                rs = self.domains().rim_sets(c, mask)
                self._bal[c] = (sum(len(a) for _, a, _ in rs.values()), sum(len(b) for _, _, b in rs.values()),
                                sum(len(a & b) for _, a, b in rs.values()))
        return self._bal[c]

    def face_caps(self):
        """U1a: caps that some single twist aligns on every misaligned inside cut cell."""
        out = []
        for c, (now, after) in self.survey().items():
            b = self.balance(c)
            if b and now - min(after.values()) >= 2 * (b[0] - b[2]) > 0:
                out.append(c)
        return out

    def face(self, c):
        """(pieces, aligned flags, side) of the face pieces of c's cut: side +1 inside, -1 outside."""
        D = self.domains()
        mask = np.zeros(rim.NP, bool)
        mask[self.inside(c)] = True
        P, A, S = [], [], []
        for d in range(len(D.reps)):
            mem = D.members[d]
            e = D.pole(d, c) if mem.size else -1
            if e < 0:
                continue
            Q = D.cham[mem]
            im = mask[mem]
            qi, pi = Q[im], mem[im]
            up = data.in_sig(qi, np.full(len(qi), e)) if qi.size else np.zeros(0, bool)
            cin = rim.partner(qi[up], np.full(int(up.sum()), e), add=False)
            okin = cin >= 0
            qo, po = Q[~im], mem[~im]
            low = ~data.in_sig(qo, np.full(len(qo), e)) if qo.size else np.zeros(0, bool)
            lo = qo[low]
            okout = rim.partner(lo, np.full(len(lo), e), add=True) >= 0
            cells_in = set(cin[okin].tolist())
            cells_out = set(lo[okout].tolist())
            if d == 0:
                cells_out.add(rim.CORE)
            for p, cell in zip(pi[up][okin].tolist(), cin[okin].tolist()):
                P.append(p); A.append(cell in cells_out); S.append(1)
            for p, cell in zip(po[low][okout].tolist(), lo[okout].tolist()):
                P.append(p); A.append(cell in cells_in); S.append(-1)
        return np.array(P, np.int64), np.array(A, bool), np.array(S, np.int64)

    def cover_graph(self):
        """{c: caps d that cover c} over the misaligned caps (U1b)."""
        sv = self.survey()
        pos = positions(self.st)
        cov = {c: set() for c in sv}
        for c in sv:
            P, A, S = self.face(c)
            if not len(P):
                continue
            for d in NEAR[c]:
                d = int(d)
                if d == c:
                    continue
                m = pos[P] @ NF[d] > KAPPA
                i = int((m & (S > 0)).sum()); o = int((m & (S < 0)).sum())
                ai = int((A & m & (S > 0)).sum()); ao = int((A & m & (S < 0)).sum())
                if i >= 10 and i == o == ai == ao:
                    cov[c].add(d)
        return cov

    def _rows(self, caps, skip=()):
        """(drop, c, items) of the caps with a lowering single twist, the largest drop first."""
        sv = self.survey()
        rows = []
        for c in caps:
            if c in skip:
                continue
            now, after = sv[c]
            low = min(after.values())
            if low < now:
                rows.append((now - low, c, [i for i, v in after.items() if v == low]))
        return sorted(rows, key=lambda t: (-t[0], t[1]))

    def cover_tops(self):
        sv = self.survey()
        cov = self.cover_graph()
        return [c for c in sv if not cov[c] and any(c in cov[x] for x in sv)]

    def candidates(self):
        """Single-twist candidates of U1a, then of U1b: (rule, c, items), the largest drop first."""
        seam = self._rows(self.face_caps())
        cov = self._rows(self.cover_tops(), {c for _, c, _ in seam})
        return [('seam', c, its) for _, c, its in seam] + [('cover', c, its) for _, c, its in cov]

    def _undo_first(self, rows):
        if not rows:
            return None
        _, c, its = rows[0]
        i = self.landing(c, its)
        self.apply(c, i)
        return [(c, i)]

    def seam(self):
        return self._undo_first(self._rows(self.face_caps()))

    def cover(self):
        return self._undo_first(self._rows(self.cover_tops()))

    # ------------------------------------------------------------------ repair
    def conj(self):
        """U5'': (c, x)(d, y)(c, z), the first that lowers N; caps in order of decreasing N_c."""
        sv = self.survey()
        mis = sorted(sv, key=lambda c: (-sv[c][0], c))
        for c in mis:
            nb = sorted(({int(x) for x in NEAR[c]} & set(mis)) - {c})
            if not nb:
                continue
            now_c, after_c = sv[c]
            for i in range(self.n_items):
                if self.g(c, i) == I4:
                    continue
                try:
                    self.apply(c, i)
                except RuntimeError:
                    continue
                d1 = after_c[i] - now_c
                for d in nb:
                    r = self.scores(d)
                    if r is None or r[0] == 0:
                        continue
                    n_d, a_d = r
                    j = min(a_d, key=lambda k: (a_d[k], k))
                    if a_d[j] >= n_d:
                        continue
                    self.apply(d, j)
                    rc = self.scores(c)
                    if rc is not None and rc[1]:
                        n_c2, a_c2 = rc
                        k = min(a_c2, key=lambda k: (a_c2[k], k))
                        if d1 + (a_d[j] - n_d) + (a_c2[k] - n_c2) < 0:
                            self.apply(c, k)
                            return [(c, i), (d, j), (c, k)]
                    self.undo()
                self.undo()
        return None

    def inv_item(self, c, i):
        g = to_tuple(transpose(self.g(c, i)))
        return next(j for j in range(self.n_items) if self.g(c, j) == g)

    def local_step(self, taboo):
        """One single-twist step of U1a, U1b, U2 or U5 on a cap not in taboo."""
        for rule, c, its in self.candidates():
            if c not in taboo:
                i = self.landing(c, its)
                self.apply(c, i)
                return rule
        sv = {c: v for c, v in self.survey().items() if c not in taboo}
        cl = self.closing(sv)
        if cl:
            _, c, its = cl[0]
            self.apply(c, self.landing(c, its))
            return 'close'
        best = None
        for c, (now, after) in sv.items():
            low = min(after.values())
            if low < now and (best is None or now - low > best[0]):
                best = (now - low, c, [i for i, v in after.items() if v == low])
        if best:
            _, c, its = best
            self.apply(c, self.landing(c, its))
            return 'improve'
        return None

    def reopen(self, K=6):
        """U7: reopen one of the solver's own earlier twists, most recent first."""
        N0 = self.misalignment()
        seen = set()
        for c, i in reversed(list(self.moves)):
            j = self.inv_item(c, i)
            if (c, j) in seen:
                continue
            seen.add((c, j))
            n0 = len(self.moves)
            try:
                self.apply(c, j)
            except RuntimeError:
                continue
            taboo = {c}
            for _ in range(K):
                if self.misalignment() < N0 or self.local_step(taboo) is None:
                    break
                taboo = set()
            if self.misalignment() < N0:
                return self.moves[n0:]
            while len(self.moves) > n0:
                self.undo()
        return None

    def run(self, max_steps=600, log=None):
        rules = {'seam': self.seam, 'cover': self.cover, 'close': self.level1, 'lock': self.level2,
                 'improve': self.improve, 'shift': self.shift, 'conj': self.conj, 'reopen': self.reopen}
        steps = []
        while len(self.moves) < max_steps:
            N = self.misalignment()
            if N == 0:
                if not self.st.is_lattice():
                    raise AssertionError('N = 0 off the lattice contradicts M3')
                return 'lattice', steps
            done = None
            for rule in self.order:
                done = rules[rule]()
                if done is not None:
                    break
            if done is None:
                return 'stuck', steps
            steps.append({'rule': rule, 'twists': done, 'N_before': N})
            if log:
                log(steps[-1])
        return 'step-limit', steps


# ---------------------------------------------------------------------- runs
def _fixture_state(name, t):
    fixture, M = wj.load(ctx, name)
    st = sim.State(ctx, menu=M)
    for rec in fixture['journal']['records'][:t]:
        if not st.apply(sim.Twist.from_record(ctx, rec)).applied:
            raise RuntimeError('fixture record not admissible')
    st.journal = []
    return st, M


ORDER = list(SeamRestorer.ORDER)


def _report(label, st, M, start_moves=(), extra=None, order=None):
    t0 = time.time()
    R = SeamRestorer(st, M, order=tuple(order or ORDER))
    for c, i in start_moves:
        R.apply(c, i)
    N0 = R.misalignment()
    t1 = time.time()

    def log(step):
        step['sec'] = round(time.time() - t1)
        print(label, json.dumps(step), file=sys.stderr, flush=True)
    res, steps = R.run(log=log)
    count = {r: sum(1 for s in steps if s['rule'] == r) for r in R.order}
    out = {'run': label, 'order': list(R.order), 'result': res, 'start_moves': len(start_moves), 'N_start': N0,
           'twists': len(R.moves), 'steps': count, 'N_end': R.misalignment(), 'grip_N_end': R.grip_misalignment(),
           'moves': R.moves, 'step_log': steps, 'seconds': round(time.time() - t0, 1)}
    if res == 'lattice':
        out['v_end'], out['v_end_in_vG'] = restore.end_invariant(st)
        out['solved'] = bool(np.array_equal(st.lattice_stickers()['labels'], np.arange(restore.NS)))
    if extra:
        out.update(extra)
    print(json.dumps({k: v for k, v in out.items() if k not in ('moves', 'step_log')}), flush=True)
    return out


def job(spec, start=None, order=None):
    """Run one job spec; write results/seams-<spec>.json; return the summary."""
    order = tuple(order or ORDER)
    parts = spec.split(':')
    kind, name = parts[0], parts[1]
    moves = json.loads(Path(start).read_text())['moves'] if start else []
    if kind == 'fixture':
        t = int(parts[2])
        st, M = _fixture_state(name, t)
        out = _report(f'{name}@{t}', st, M, moves, order=order)
    elif kind == 'random':
        k, seed = int(parts[2]), int(parts[3])
        mix = float(parts[4]) if len(parts) > 4 else 0.0
        M = restore.menu(name)
        st, hist = restore.scramble(M, k, seed, mix)
        out = _report(f'{name} k={k}{" mix=" + str(mix) if mix else ""} seed={seed}', st, M, moves, {'history': hist}, order)
    else:
        raise SystemExit(f'unknown job {spec}')
    tag = spec.replace(':', '-') + ('-continued' if start else '') + ('' if order == SeamRestorer.ORDER else '-order-' + '-'.join(order))
    (HERE / 'results' / f'seams-{tag}.json').write_text(json.dumps(out, indent=1) + '\n')
    return {k: v for k, v in out.items() if k not in ('moves', 'step_log')}


def facecheck(name, ts):
    """Face match against top-record labels at the true scramble states X_t (journal used for labels only)."""
    fixture, M = wj.load(ctx, name)
    tw = [sim.Twist.from_record(ctx, r) for r in fixture['journal']['records'][:max(ts)]]
    st = sim.State(ctx, menu=M)
    rows, done = [], 0

    def replay(keep):
        s = sim.State(ctx, menu=M)
        for k in keep:
            if not s.apply(tw[k]).applied:
                return None
        return s.digest()
    for t in ts:
        for x in tw[done:t]:
            assert st.apply(x).applied
        done = t
        st.journal = []
        R = SeamRestorer(st, M)
        sv = R.survey()
        passes = set(R.face_caps())
        latest = {}
        for k in range(t):
            latest[tw[k].grip] = k
        tp = fp = fn = 0
        for c in sv:
            if c not in latest:
                fp += c in passes
                continue
            k = latest[c]
            o = st.apply(sim.Twist(ctx, c, to_tuple(transpose(tw[k].matrix))))
            top = False
            if o.applied:
                d = st.digest()
                st.undo()
                top = replay([j for j in range(t) if j != k]) == d
            tp += top and c in passes
            fp += (not top) and c in passes
            fn += top and c not in passes
        rows.append({'t': t, 'misaligned_caps': len(sv), 'true_positive': tp, 'false_positive': fp, 'miss': fn})
        print(json.dumps(rows[-1]), flush=True)
    (HERE / 'results' / f'seams-facecheck-{name}.json').write_text(json.dumps(rows, indent=1) + '\n')


def main(argv):
    if not argv:
        raise SystemExit(__doc__)
    if '--order' in argv:
        k = argv.index('--order')
        ORDER[:] = argv[k + 1].split(',')
        argv = argv[:k] + argv[k + 2:]
    mode = argv[0]
    if mode == 'fixture':
        start = argv[argv.index('--start') + 1] if '--start' in argv else None
        job(f'fixture:{argv[1]}:{argv[2]}', start)
    elif mode == 'random':
        mix = argv[4] if len(argv) > 4 else None
        for seed in argv[3].split(','):
            job(f'random:{argv[1]}:{argv[2]}:{seed}' + (f':{mix}' if mix else ''))
    elif mode == 'batch':
        jobs = argv[1:]
        n = os.cpu_count() or 1
        if jobs and jobs[0] == '--jobs':
            n, jobs = int(jobs[1]), jobs[2:]
        home_centroids()        # fill the cache once before the workers start
        with Pool(min(n, len(jobs))) as pool:
            for summary in pool.imap_unordered(partial(job, order=tuple(ORDER)), jobs):
                print('done', json.dumps(summary), flush=True)
    elif mode == 'facecheck':
        facecheck(argv[1], [int(x) for x in argv[2].split(',')])
    else:
        raise SystemExit(__doc__)


if __name__ == '__main__':
    main(sys.argv[1:])
