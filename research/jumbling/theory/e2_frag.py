"""Experiment E2: restoration with no solver history, one potential P = (off-lattice count, N) in
lexicographic order, and a fragment rule (lens commutator [(d,a),(c,q)]).
Every accepted step must lower P. Rules: seam, cover, close, carrier, lock, conj, comm, improve, shift.
Usage: python research/jumbling/theory/e2_frag.py <menu> <t> <moves.json> <out.json> [--prefix k] [--max 200] [--keep-history]
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
ap.add_argument('menu'); ap.add_argument('t', type=int); ap.add_argument('moves'); ap.add_argument('out')
ap.add_argument('--prefix', type=int); ap.add_argument('--max', type=int, default=200)
ap.add_argument('--keep-history', action='store_true'); ap.add_argument('--T', type=int, default=1549)
ap.add_argument('--order', default='seam,cover,close,carrier,lock,conj,comm,improve,shift')
a_ = ap.parse_args()
kp = seams.ctx.kplus
ORDER = tuple(a_.order.split(','))


class E2(seams.SeamRestorer):
    def in_k(self, m):
        return kp.index_of_matrix(to_tuple(m)) >= 0

    def off(self):
        return int((self.domains().dom != 0).sum())

    def P(self):
        return (self.off(), self.misalignment())

    def carrier(self):
        D = self.domains()
        sizes = [len(m) for m in D.members]
        best = None
        for c in range(600):
            ins = self.inside(c)
            if ins is None:
                continue
            us, cnt = np.unique(D.dom[ins], return_counts=True)
            ds = [int(d) for d, k in zip(us, cnt) if d != 0 and sizes[d] >= a_.T and k == sizes[d]]
            if not ds:
                continue
            for i in range(self.n_items):
                g = self.g(c, i)
                if g == seams.I4 or not any(self.in_k(matmul(g, D.reps[d])) for d in ds):
                    continue
                dlt = sum(int(k) * ((0 if self.in_k(matmul(g, D.reps[e])) else 1) - (0 if e == 0 else 1))
                          for e, k in zip(us.tolist(), cnt.tolist()))
                if dlt < 0 and (best is None or dlt < best[0]):
                    best = (dlt, c, i)
        if best is None:
            return None
        self.apply(best[1], best[2])
        return [(best[1], best[2])]

    def comm(self):
        """Fragment rule: for each off-lattice domain F (largest first) and each cap pair (d, c) with F inside
        the lens of c and d (d holding F), the commutator (d,a)(c,q)(d,a^-1)(c,q^-1) with a.gamma_F in K+;
        the word with the lowest P among the first pair that lowers P."""
        P0 = self.P()
        D = self.domains()
        doms = sorted(((d, set(np.asarray(D.members[d]).tolist()), D.reps[d]) for d in range(1, len(D.members))),
                      key=lambda x: -len(x[1]))
        sets = {c: set(np.asarray(self.inside(c)).tolist()) for c in range(600) if self.inside(c) is not None}
        for fd, F, gF in doms:
            holders = [d for d, s in sets.items() if F <= s]
            touching = [c for c, s in sets.items() if F & s]
            pairs = sorted((len(sets[c] & sets[d]) - len(F), d, c) for d in holders for c in touching
                           if c != d and F <= sets[c] & sets[d])
            for _, d, c in pairs:
                best = None
                for a in range(self.n_items):
                    if self.g(d, a) == seams.I4 or not self.in_k(matmul(self.g(d, a), gF)):
                        continue
                    for q in range(self.n_items):
                        if self.g(c, q) == seams.I4:
                            continue
                        word = [(d, a), (c, q), (d, self.inv_item(d, a)), (c, self.inv_item(c, q))]
                        n = 0
                        try:
                            for w in word:
                                self.apply(*w); n += 1
                        except RuntimeError:
                            for _ in range(n):
                                self.undo()
                            continue
                        o = self.off()
                        if o <= P0[0]:
                            p = (o, self.misalignment())
                            if p < P0 and (best is None or p < best[0]):
                                best = (p, word)
                        for _ in range(4):
                            self.undo()
                if best:
                    for w in best[1]:
                        self.apply(*w)
                    return best[1]
        return None

    def run(self, max_twists, log):
        rules = {'seam': self.seam, 'cover': self.cover, 'close': self.level1, 'carrier': self.carrier,
                 'lock': self.level2, 'conj': self.conj, 'comm': self.comm, 'improve': self.improve,
                 'shift': self.shift}
        steps, n0 = [], len(self.moves)
        seen = {self.st.digest()}
        while True:
            P0 = self.P()
            if P0[1] == 0:
                assert self.st.is_lattice()
                return 'lattice', steps
            if len(self.moves) - n0 >= max_twists:
                return 'step-limit', steps
            done = None
            for rule in self.order:
                k = len(self.moves)
                out = rules[rule]()
                if out is None:
                    continue
                if self.P() < P0 and self.st.digest() not in seen:
                    done = out
                    break
                while len(self.moves) > k:
                    self.undo()
            if done is None:
                return 'stuck', steps
            seen.add(self.st.digest())
            steps.append({'rule': rule, 'twists': done, 'P_before': list(P0)})
            log(steps[-1])


st, M = seams._fixture_state(a_.menu, a_.t)
R = E2(st, M, order=ORDER)
start = json.load(open(a_.moves))['moves']
if a_.prefix is not None:
    start = start[:a_.prefix]
for c, i in start:
    R.apply(c, i)
if not a_.keep_history:
    R.moves = []          # a fresh solver: the configuration only, no own record
t0 = time.time()
print(json.dumps({'start': len(start), 'history': len(R.moves), 'P': list(R.P())}), flush=True)


def log(step):
    step['P_after'] = list(R.P())
    step['sec'] = round(time.time() - t0)
    print(json.dumps(step), flush=True)


res, steps = R.run(a_.max, log)
out = {'run': f'{a_.menu}@{a_.t}', 'experiment': 'E2 fragment rule, potential (off, N)', 'order': ORDER,
       'result': res, 'start_moves': len(start), 'history_kept': a_.keep_history, 'P_end': list(R.P()),
       'moves': R.moves, 'step_log': steps, 'seconds': round(time.time() - t0)}
json.dump(out, open(a_.out, 'w'))
print(json.dumps({k: v for k, v in out.items() if k not in ('moves', 'step_log')}), flush=True)
