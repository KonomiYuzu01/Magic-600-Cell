"""J3-R restoration procedure, phase U (restoration.md, section 3): rim descent.

The procedure reads only the configuration: poses, and through them domains and admissibility.
It never reads a journal.
- Level 1: among the grips whose cut is misaligned (N_c > 0), take one that a single menu twist
  closes (N_c = 0 afterwards), and make that twist. The largest misalignment goes first; among the
  closing twists of a grip, the one that brings the most pieces home.
- Level 2, only when level 1 finds nothing: open a misaligned grip c by one menu twist, close
  another misaligned grip that this makes closable, then close any grip that can close. The three
  twists are kept only if the total misalignment N ends lower than before.
By restoration.md M2 every kept step lowers N, so the procedure cannot cycle. It ends on the
lattice (N = 0, M3) or reports 'stuck'.

Run from the repository root (results go to results/restore-<mode>-<menu>.json):
    python research/jumbling/theory/restore.py random <menu> <k> <seeds> [mix]  # k random twists
    python research/jumbling/theory/restore.py fixture <menu> <t>         # first t W-J records
    python research/jumbling/theory/restore.py lens <menu> [limit]        # (0,q)(d,a)(0,q^-1) words
Menus: S4, I_a, I_b. Seeds: comma-separated integers. mix: probability that a scramble twist is
drawn from the whole menu (retained elements included) instead of the jumble elements only.
A run that ends on the lattice also reports the invariant v of the end configuration (lens.py) and
whether it lies in v(G) = {0, V, 2V}, V the common value of the retained generators.

Evidence kind: source and exact synthetic geometry. Timings are cloud timings of research code.
"""
import json
import random
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))

import rim  # noqa: E402
import sim  # noqa: E402
import trees  # noqa: E402
import wj  # noqa: E402
from exact import identity, matmul, transpose  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402
from sim.model import NS  # noqa: E402

ctx = rim.ctx
_NF = np.array([[float(v) for v in row] for row in ctx.data.N])
_ANG = np.degrees(np.arccos(np.clip(_NF @ _NF.T / float(ctx.data.NN), -1, 1)))
NEAR = [np.nonzero(_ANG[d] < 55.7)[0] for d in range(600)]   # caps can meet below 2 theta = 52.67 deg
_MENUS = {}


def menu(name):
    if name not in _MENUS:
        _MENUS[name] = wj.make_menu(ctx, name)
    return _MENUS[name]


def element(M, c, i):
    f = M.frame(c)
    return to_tuple(matmul(matmul(f, M.items[i][1]), transpose(f)))


def jumble_items(M):
    stab = [ctx.kplus.matrix(s) for s in ctx.kplus.stab0]
    return [i for i, (_, m) in enumerate(M.items) if m not in stab]


def scramble(M, k, seed, retained_mix=0.0):
    """k admissible twists at uniformly random grips (inadmissible draws are skipped): a jumble
    twist, or with probability retained_mix any menu element (retained ones included)."""
    rng = random.Random(seed)
    st = sim.State(ctx, menu=M)
    J = jumble_items(M)
    every = list(range(len(M.items)))
    hist = []
    while len(hist) < k:
        c = rng.randrange(600)
        i = rng.choice(every if rng.random() < retained_mix else J)
        if st.apply(sim.Twist(ctx, c, element(M, c, i), 'menu', {'menu': M.name})).applied:
            hist.append((c, i))
    st.journal = []
    return st, hist


class Restorer:
    def __init__(self, st, M):
        self.st, self.M = st, M
        self.n_items = len(M.items)
        self._cls = {}
        self._sv = {}
        self._dom = None
        self._g = {}
        self._tree = {}
        self.moves = []

    # ---------------------------------------------------------------- state access with caches
    def g(self, c, i):
        if (c, i) not in self._g:
            self._g[(c, i)] = element(self.M, c, i)
        return self._g[(c, i)]

    def inside(self, c):
        if c not in self._cls:
            cl = self.st.classify(int(c), need_inside=True, stop_at_straddle=True)
            ok = cl.straddle is None and not cl.uncertain
            self._cls[c] = cl.inside if ok else None
        return self._cls[c]

    def domains(self):
        if self._dom is None:
            self._dom = rim.Domains(self.st)
        return self._dom

    def _touch(self, c):
        self._dom = None
        for f in NEAR[int(c)]:
            self._cls.pop(int(f), None)
            self._sv.pop(int(f), None)

    def apply(self, c, i):
        out = self.st.apply(sim.Twist(ctx, int(c), self.g(c, i), 'menu', {'menu': self.M.name}))
        if not out.applied:
            raise RuntimeError(f'twist ({c}, {i}) not admissible: {out.status}')
        self._touch(c)
        self.moves.append((int(c), int(i)))

    def undo(self):
        c = self.st.journal[-1]['grip']
        self.st.undo()
        self._touch(c)
        self.moves.pop()

    # ---------------------------------------------------------------- misalignment
    def scores(self, c):
        """(N_c, {item: N_c after the twist}) for an admissible grip c, else None."""
        if c not in self._sv:
            ins = self.inside(c)
            if ins is None:
                self._sv[c] = None
            else:
                D = self.domains()
                mask = np.zeros(rim.NP, bool)
                mask[ins] = True
                rs = D.rim_sets(c, mask)
                now = D.misalignment(c, mask, rs=rs)
                after = {} if now == 0 else {
                    i: D.misalignment(c, mask, (c, i), self.g(c, i), rs) for i in range(self.n_items)}
                self._sv[c] = (now, after)
        return self._sv[c]

    def survey(self, caps=range(600)):
        out = {}
        for c in caps:
            r = self.scores(int(c))
            if r is not None and r[0] > 0:
                out[int(c)] = r
        return out

    def misalignment(self):
        return sum(v[0] for v in self.survey().values())

    def height(self):
        """H = sum over pieces of d_L + d_R (reporting only)."""
        tot = np.bincount(self.st.pose_id, minlength=len(self.st._poses))
        H = 0
        for pid in np.nonzero(tot)[0].tolist():
            p = self.st._poses[pid]
            if p.kidx >= 0:
                continue
            if p.key not in self._tree:
                dl, dr, _, _ = trees.pose_tree(self.st._pose_matrix(pid))
                self._tree[p.key] = dl + dr
            H += int(tot[pid]) * self._tree[p.key]
        return H

    # ---------------------------------------------------------------- the procedure
    @staticmethod
    def closing(sv):
        out = [(now, c, [i for i, v in after.items() if v == 0]) for c, (now, after) in sv.items()]
        return sorted([(n, c, its) for n, c, its in out if its], key=lambda t: (-t[0], t[1]))

    def landing(self, c, its):
        """Among the closing twists its of grip c, the one that brings the most inside pieces home
        (pose the identity), the lowest menu index on a tie. Labels are visible, so this is a rule a
        solver can follow; it matters when the interior is all lattice and every landing closes."""
        ins = self.inside(c)
        pids, counts = np.unique(self.st.pose_id[ins], return_counts=True)
        I4 = to_tuple(identity(4))
        mats = [self.st._pose_matrix(int(p)) for p in pids]

        def home(i):
            g = self.g(c, i)
            return sum(int(n) for M, n in zip(mats, counts) if to_tuple(matmul(g, M)) == I4)
        return max(its, key=lambda i: (home(i), -i))

    def level1(self):
        cl = self.closing(self.survey())
        if not cl:
            return None
        _, c, its = cl[0]
        i = self.landing(c, its)
        self.apply(c, i)
        return [(c, i)]

    def level2(self):
        """Open a misaligned grip c, close a misaligned grip d that this makes closable, then close
        a grip e. By M2 the total misalignment changes only on the cut being twisted, so the end
        value is N0 + (N_c after the opening - N_c) - N_d - N_e, without a full survey."""
        sv = self.survey()
        N0 = sum(v[0] for v in sv.values())
        mis = sorted(sv)
        best = None
        for c in mis:
            now_c, after_c = sv[c]
            for i in range(self.n_items):
                if after_c[i] == now_c:
                    continue                      # leaves the cut as it is: not an opening
                try:
                    self.apply(c, i)
                except RuntimeError:
                    continue
                d_open = after_c[i] - now_c
                for n_d, d, its in self.closing(self.survey([x for x in mis if x != c]))[:2]:
                    self.apply(d, its[0])
                    cl = self.closing(self.survey(sorted(set(mis) | {c})))
                    if cl:
                        n_e, e, its2 = cl[0]
                        N = N0 + d_open - n_d - n_e
                        if N < N0 and (best is None or N < best[0]):
                            best = (N, [(c, i), (d, its[0]), (e, self.landing(e, its2))])
                    self.undo()
                self.undo()
        if best is None:
            return None
        for c, i in best[1]:
            self.apply(c, i)
        return best[1]

    def run(self, max_steps=400, log=None):
        steps = []
        while len(self.moves) < max_steps:
            N = self.misalignment()
            if N == 0:
                return ('lattice' if self.st.is_lattice() else 'closed-off-lattice'), steps
            done = self.level1()
            level = 1
            if done is None:
                done = self.level2()
                level = 2
            if done is None:
                return 'stuck', steps
            steps.append({'level': level, 'twists': done, 'N_before': N})
            if log:
                log(steps[-1])
        return 'step-limit', steps


_VG = None


def end_invariant(st):
    """(v of a lattice configuration as a sorted list, whether it lies in v(G))."""
    global _VG
    import lens
    if lens.AB is None:
        lens.AB = lens.abelianisations()
    if _VG is None:
        g = sim.State(ctx)
        g.apply(sim.a4_element(ctx, 0, 4))           # a third-turn (item 1 is a half-turn, v = 0)
        V = lens.v(g._kpose()[g.pose_id])
        _VG = [{}, V, {r: (0, lens.AB[int(lens.orbit_of[np.nonzero(lens.ROB == r)[0][0]])]['mult'][(a, a)])
                       for r, (p, a) in V.items()}]
    val = lens.v(st._kpose()[st.pose_id])
    return (None if val is None else sorted(val.items())), val in _VG


def _report(label, st, M, t0, extra=None):
    R = Restorer(st, M)
    N0, H0 = R.misalignment(), R.height()
    res, steps = R.run()
    out = {'run': label, 'result': res, 'N_start': N0, 'H_start': H0, 'twists': len(R.moves),
           'level2_macros': sum(1 for s in steps if s['level'] == 2), 'N_end': R.misalignment(),
           'H_end': R.height(), 'moves': R.moves, 'seconds': round(time.time() - t0, 1)}
    if res == 'lattice':
        out['v_end'], out['v_end_in_vG'] = end_invariant(st)
        out['solved'] = bool(np.array_equal(st.lattice_stickers()['labels'], np.arange(NS)))
        out['moved_end'] = st.moved_count()
    if extra:
        out.update(extra)
    print(json.dumps({k: v for k, v in out.items() if k != 'moves'}), flush=True)
    return out


def main(argv):
    mode, name = argv[0], argv[1]
    M = menu(name)
    results = []
    if mode == 'random':
        k = int(argv[2])
        mix = float(argv[4]) if len(argv) > 4 else 0.0
        tag = f' mix={mix}' if mix else ''
        for seed in [int(s) for s in argv[3].split(',')]:
            t0 = time.time()
            st, hist = scramble(M, k, seed, mix)
            results.append(_report(f'{name} k={k}{tag} seed={seed}', st, M, t0, {'history': hist}))
    elif mode == 'fixture':
        t = int(argv[2])
        t0 = time.time()
        fixture, M = wj.load(ctx, name)
        st = sim.State(ctx, menu=M)
        for rec in fixture['journal']['records'][:t]:
            if not st.apply(sim.Twist.from_record(ctx, rec)).applied:
                raise RuntimeError('fixture record not admissible')
        st.journal = []
        results.append(_report(f'{name}@{t}', st, M, t0))
    elif mode == 'lens':
        limit = int(argv[2]) if len(argv) > 2 else 10**9
        J = jumble_items(M)
        A4 = [i for i in range(len(M.items)) if i not in J]
        target = 90 if name == 'S4' else 72

        def angle(m):
            tr = sum(float(m[k][k]) for k in range(4))
            return round(float(np.degrees(np.arccos(max(-1, min(1, (tr - 2) / 2))))))
        qi = next(i for i in J if angle(element(M, 0, i)) == target)
        qinv = to_tuple(transpose(element(M, 0, qi)))
        I4 = to_tuple(identity(4))
        n = 0
        for d in range(1, 600):
            for a in A4:
                if n >= limit or element(M, d, a) == I4:
                    continue
                t0 = time.time()
                st = sim.State(ctx, menu=M)
                ok = all(st.apply(sim.Twist(ctx, c, m)).applied
                         for c, m in ((0, element(M, 0, qi)), (d, element(M, d, a)), (0, qinv)))
                if not ok or st.is_lattice():
                    continue
                st.journal = []
                n += 1
                results.append(_report(f'{name} lens d={d} a={a}', st, M, t0, {'word': [(0, qi), (d, a), (0, 'q^-1')]}))
    else:
        raise SystemExit(__doc__)
    path = HERE / 'results' / f'restore-{mode}-{name}.json'
    old = json.loads(path.read_text()) if path.exists() else []
    path.write_text(json.dumps(old + results, indent=1) + '\n')


if __name__ == '__main__':
    if len(sys.argv) < 3:
        print(__doc__)
    else:
        main(sys.argv[1:])
