# Stage 14 (blocking structure), tool 3: preparations by retained twists, in the label calculus.
#
# Lemma L (stage README): a retained twist (c, k), k in A4_c, at a clean grip c moves exactly the pieces whose label at c
# is "in", and for each of them the new label at grip d is its old label at k^-1(d); heights do not change, and the
# branch of a moved off piece becomes s_k(branch), where s_k is the permutation of the five classes induced by k
# (class of k(e) = s_k(class of e)). So the dynamics of retained twists is a permutation action on the label table,
# exact without any geometry.
#
# For each pose group G of height 4 or more, the search looks for a word of retained twists at clean grips after which
# some own-branch cap x holds G whole, is clean, and is pure (holds no piece of another branch at height 2 or more), so
# that a lowering twist at x (12 menu items lower G by 2, R21F-03) raises no height above 2. Best-first beam search on
# cost(x) = straddlers of x + foreign load of x, minimised over the own holders x of G. Every reported word is replayed
# on the exact simulator at the end (each twist admissible, the claimed straddler count and foreign load re-measured).
# Exact geometry for the start labels and the replay; research only, headless Linux.
#
# Run from the repository root:
#   python3 -I -B research/jumbling/restoration/14-blocking-structure/scripts/retained_search.py <table.json>
#       <labels.npz> <out.json> [--depth D] [--beam B] [--groups g,g,...] [--no-replay]
import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import blockmap as BM  # noqa: E402

C, M, KP, CLS, sim, R, T = BM.C, BM.M, BM.KP, BM.CLS, BM.sim, BM.R, BM.T
PERMS = KP.perms.astype(np.int64)
INV = np.asarray(KP.inv, np.int64)
REP = np.array([int(np.flatnonzero(CLS == q)[0]) for q in range(5)])


def sigma(k):
    """Permutation of the five classes (and branches) induced by the K+ element k; index 5 maps lattice (-1) to -1."""
    s = np.full(6, -1, np.int64)
    s[:5] = CLS[PERMS[k][REP]]
    return s


class LabelState:
    def __init__(self, lab, h, b):
        self.L = lab.copy()
        self.h = h.astype(np.int64)
        self.b = b.astype(np.int64)

    def clean(self):
        return ~(self.L == 2).any(0)

    def apply(self, c, k):
        rows = np.flatnonzero(self.L[:, c] == 1)
        pre = PERMS[INV[k]]
        self.L[rows] = self.L[rows][:, pre]
        s = sigma(k)
        self.b[rows] = s[self.b[rows]]          # index -1 reads s[-1] = s[5] = -1

    def foreign_inside(self, x):
        return int(((self.L[:, x] == 1) & (self.h >= 2) & (self.b != CLS[x])).sum())

    def own_holders(self, G):
        Bg = int(self.b[G[0]])
        return np.flatnonzero((self.L[G] == 1).all(0) & (CLS == Bg))

    def cost(self, G):
        best = None
        for x in self.own_holders(G).tolist():
            v = (int((self.L[:, x] == 2).sum()) + self.foreign_inside(x), x)
            best = v if best is None or v < best else best
        return best if best is not None else (10 ** 9, -1)

    def cost_after(self, G, c, k, nS, nF):
        """cost(G) after the retained twist (c, k), from the incremental formulas of Lemma L (no copy)."""
        rows = np.flatnonzero(self.L[:, c] == 1)
        inR = np.zeros(len(self.h), bool)
        inR[rows] = True
        pre = PERMS[INV[k]]
        s = sigma(k)
        if inR[G].any():
            if not inR[G].all():
                return (10 ** 9, -1)
            labG = self.L[G][:, pre]
            Bg = int(s[self.b[G[0]]])
        else:
            labG = self.L[G]
            Bg = int(self.b[G[0]])
        hold = np.flatnonzero((labG == 1).all(0) & (CLS == Bg))
        if not len(hold):
            return (10 ** 9, -1)
        Lr = self.L[rows]
        hr, br = self.h[rows], s[self.b[rows]]
        best = None
        for x in hold.tolist():
            px = int(pre[x])
            strad = nS[x] - int((Lr[:, x] == 2).sum()) + int((Lr[:, px] == 2).sum())
            old_f = int(((Lr[:, x] == 1) & (hr >= 2) & (self.b[rows] != CLS[x])).sum())
            new_f = int(((Lr[:, px] == 1) & (hr >= 2) & (br != CLS[x])).sum())
            v = (strad + nF[x] - old_f + new_f, x)
            best = v if best is None or v < best else best
        return best


def search(ls0, G, depth, beam, deadline):
    start = ls0.cost(G)
    frontier = [(start, [], ls0)]
    best = (start, [])
    for _ in range(depth):
        cand = []
        for (cst, word, ls) in frontier:
            S2 = ls.L == 2
            nS = S2.sum(0)
            In = ls.L == 1
            nF = np.zeros(600, np.int64)
            for q in range(5):
                nF[CLS == q] = In[(ls.h >= 2) & (ls.b != q)][:, CLS == q].sum(0)
            for c in np.flatnonzero(nS == 0).tolist():
                for k in KP.a4(c):
                    if k == 0:
                        continue
                    if time.time() > deadline:
                        return best, 'deadline'
                    v = ls.cost_after(G, c, int(k), nS, nF)
                    cand.append((v, word + [(c, int(k))], ls))
                    if v < best[0]:
                        best = (v, word + [(c, int(k))])
                        if v[0] == 0:
                            return best, 'found'
        cand.sort(key=lambda r: r[0])
        nxt = []
        seen = set()
        for v, word, ls in cand:
            if len(nxt) >= beam:
                break
            t = LabelState(ls.L, ls.h, ls.b)
            t.apply(*word[-1])
            key = hash(t.L.tobytes())
            if key in seen:
                continue
            seen.add(key)
            nxt.append((v, word, t))
        frontier = nxt
        if not frontier:
            break
    return best, 'exhausted'


def replay(st, word, x, G):
    """Apply the word exactly, re-measure straddlers of x and its foreign load, then undo."""
    from exact import transpose
    from sim.kplus import to_tuple
    import jamcheck as J
    d0 = st.digest()
    done = []
    ok = True
    for c, k in word:
        g = to_tuple(KP.matrix(k))
        o = st.apply(sim.Twist(C, c, g, 'menu', {'menu': M.name}), journal=False)
        assert not o.uncertain
        if not o.applied:
            ok = False
            break
        done.append((c, g))
    res = dict(admissible=ok)
    if ok:
        h, b = BM.tree_info(st)
        S = J.exact_straddlers(st, x)
        lab_x = BM._label_caps(st, [x], np.flatnonzero(st._kpose()[st.pose_id] >= 0), st._kpose()[st.pose_id],
                               [(pid, st._anchors(pid, mem)) for pid, mem in
                                st._groups(np.flatnonzero(st._kpose()[st.pose_id] < 0))])[:, 0]
        res.update(straddlers=int(len(S)),
                   foreign=int(((lab_x == 1) & (h >= 2) & (b != CLS[x])).sum()),
                   group_inside=bool((lab_x[G] == 1).all()), group_heights=sorted(set(h[G].tolist())))
    for c, g in reversed(done):
        assert st.apply(sim.Twist(C, c, to_tuple(transpose(g)), 'menu', {'menu': M.name}), journal=False).applied
    assert st.digest() == d0
    return res


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('table')
    ap.add_argument('labels')
    ap.add_argument('out')
    ap.add_argument('--depth', type=int, default=6)
    ap.add_argument('--beam', type=int, default=8)
    ap.add_argument('--seconds', type=float, default=600)
    ap.add_argument('--groups')
    ap.add_argument('--no-replay', action='store_true')
    a = ap.parse_args(argv)
    st = BM.load_table(a.table)
    h, b = BM.tree_info(st)
    lab = BM.labels_for(st, a.labels, 1)
    off = np.flatnonzero(h > 0)
    pos = np.full(len(h), -1, np.int64)
    pos[off] = np.arange(len(off))
    ls0 = LabelState(lab[off], h[off], b[off])
    groups = sorted(np.unique(st.pose_id[h >= 4]).tolist())
    if a.groups:
        groups = [int(g) for g in a.groups.split(',')]
    out = []
    for g in groups:
        mem = np.flatnonzero(st.pose_id == g)
        G = pos[mem]
        t0 = time.time()
        (cst, word), status = search(ls0, G, a.depth, a.beam, time.time() + a.seconds)
        row = dict(pose=int(g), size=int(len(mem)), height=int(h[mem[0]]), branch=int(b[mem[0]]),
                   start=[int(v) for v in ls0.cost(G)], best=[int(v) for v in cst], status=status, word=[[int(c), int(k)] for c, k in word],
                   seconds=round(time.time() - t0, 1))
        if word and not a.no_replay:
            row['replay'] = replay(st, word, cst[1], mem)
        out.append(row)
        print(json.dumps({k: row[k] for k in ('pose', 'size', 'start', 'best', 'status', 'seconds')}
                         | ({'replay': row['replay']} if 'replay' in row else {}) | {'len': len(word)}), flush=True)
    Path(a.out).write_text(json.dumps(dict(digest=st.digest(), groups=out), indent=1))


if __name__ == '__main__':
    main()
