"""Joint-attack experiment E1: carrier return before repair words, no retained-only words.
carrier: a single twist (c, i) after which a whole off-lattice domain of at least T pieces, all inside c,
is on the lattice, and the off-lattice piece count falls; the largest fall first, taken even if N rises.
Retained-only words (every twist in K+) from lock, conj, shift and reopen are rejected.
Usage: python research/jumbling/theory/e1_carrier.py <menu> <t> <out.json> [--start <moves.json> [--prefix k]] [--T 1549] [--max 400]
Run from the repository root. Evidence kind: source and exact synthetic geometry; timings are cloud
timings of research code, not performance evidence."""
import sys, json, time, argparse
import numpy as np
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import seams
from exact import matmul
from sim.kplus import to_tuple

ap = argparse.ArgumentParser()
ap.add_argument('menu'); ap.add_argument('t', type=int); ap.add_argument('out')
ap.add_argument('--start'); ap.add_argument('--prefix', type=int)
ap.add_argument('--T', type=int, default=1549); ap.add_argument('--max', type=int, default=400)
a = ap.parse_args()
kp = seams.ctx.kplus
ORDER = ('seam', 'cover', 'close', 'carrier', 'lock', 'conj', 'improve', 'shift', 'reopen')


class Carrier(seams.SeamRestorer):
    def in_k(self, m):
        return kp.index_of_matrix(to_tuple(m)) >= 0

    def retained(self, c, i):
        return self.in_k(self.g(c, i))

    def off(self):
        return int((self.domains().dom != 0).sum())

    def carrier(self):
        D = self.domains()
        sizes = [len(m) for m in D.members]
        best = None
        for c in range(600):
            ins = self.inside(c)
            if ins is None:
                continue
            dom_in = D.dom[ins]
            us, cnt = np.unique(dom_in, return_counts=True)
            ds = [int(d) for d, k in zip(us, cnt) if d != 0 and sizes[d] >= a.T and k == sizes[d]]
            if not ds:
                continue
            for i in range(self.n_items):
                g = self.g(c, i)
                if g == seams.I4 or not any(self.in_k(matmul(g, D.reps[d])) for d in ds):
                    continue
                dlt = 0
                for e, k in zip(us.tolist(), cnt.tolist()):
                    on_after = self.in_k(matmul(g, D.reps[e]))
                    dlt += int(k) * ((0 if on_after else 1) - (0 if e == 0 else 1))
                if dlt < 0 and (best is None or dlt < best[0]):
                    best = (dlt, c, i)
        if best is None:
            return None
        try:
            self.apply(best[1], best[2])
        except RuntimeError:
            return None
        return [(best[1], best[2])]

    def _no_retained_only(self, fn):
        n0 = len(self.moves)
        out = fn()
        if out and all(self.retained(c, i) for c, i in out):
            while len(self.moves) > n0:
                self.undo()
            return None
        return out

    def conj(self):
        """U5'' with retained-only words skipped (searched past, not given up)."""
        sv = self.survey()
        mis = sorted(sv, key=lambda c: (-sv[c][0], c))
        for c in mis:
            nb = sorted(({int(x) for x in seams.NEAR[c]} & set(mis)) - {c})
            if not nb:
                continue
            now_c, after_c = sv[c]
            for i in range(self.n_items):
                if self.g(c, i) == seams.I4:
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
                        if d1 + (a_d[j] - n_d) + (a_c2[k] - n_c2) < 0 and not (
                                self.retained(c, i) and self.retained(d, j) and self.retained(c, k)):
                            self.apply(c, k)
                            return [(c, i), (d, j), (c, k)]
                    self.undo()
                self.undo()
        return None

    def run(self, max_twists=400, log=None):
        rules = {'seam': self.seam, 'cover': self.cover, 'close': self.level1, 'carrier': self.carrier,
                 'lock': lambda: self._no_retained_only(self.level2), 'improve': self.improve,
                 'shift': lambda: self._no_retained_only(self.shift), 'conj': self.conj,
                 'reopen': lambda: self._no_retained_only(self.reopen)}
        steps, n0 = [], len(self.moves)
        while True:
            N = self.misalignment()
            if N == 0:
                assert self.st.is_lattice()
                return 'lattice', steps
            if len(self.moves) - n0 >= max_twists:
                return 'step-limit', steps
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


st, M = seams._fixture_state(a.menu, a.t)
R = Carrier(st, M, order=ORDER)
start = []
if a.start:
    start = json.load(open(a.start))['moves']
    if a.prefix is not None:
        start = start[:a.prefix]
for c, i in start:
    R.apply(c, i)
t0 = time.time()
print(json.dumps({'start': len(start), 'N': R.misalignment(), 'off': R.off()}), flush=True)


def log(step):
    step['N_after'] = R.misalignment()
    step['off'] = R.off()
    step['sec'] = round(time.time() - t0)
    print(json.dumps(step), flush=True)


res, steps = R.run(a.max, log)
out = {'run': f'{a.menu}@{a.t}', 'experiment': 'E1 carrier', 'order': ORDER, 'T': a.T, 'result': res,
       'start_moves': len(start), 'N_end': R.misalignment(), 'off_end': R.off(), 'twists': len(R.moves) - len(start),
       'moves': R.moves, 'step_log': steps, 'seconds': round(time.time() - t0)}
json.dump(out, open(a.out, 'w'))
print(json.dumps({k: v for k, v in out.items() if k not in ('moves', 'step_log')}), flush=True)
