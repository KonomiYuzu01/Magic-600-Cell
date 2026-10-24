# Stage 14 (blocking structure), tool 5: peel the obstruction of a target grip through two of its holders, then lower.
#
# For a target grip x and two clean grips A and B (for example clean pure holders of the obstruction of x: its
# straddlers and the pieces of another branch at height 2 or more inside it), every non-identity menu item a at A is
# applied exactly and the straddlers S and foreign load F of x are measured; the `--keep` first letters with the
# smallest S + F (ties by S, then a) are each followed by every non-identity item b at B (B must be clean after the
# first letter, or the letter is not admissible and is recorded so). When x is clean after the two letters, every
# non-identity item j at x is applied, and the exact change of the depth Phi = sum of max(0, h - 2), of n4 and the
# largest height are recorded for the best j (smallest change of Phi, then of n4, then largest height, then j).
# Only the pieces inside a cap move under its twist (theory draft, Proposition 2.2), so heights, branches and labels
# at x are recomputed exactly for those pieces alone; every twist is undone by its inverse and the state digest is
# checked at the end. A word found here is a lead until replay_word.py replays it from the table.
# Exact geometry; research only, headless Linux; no Windows, DirectX, input or performance claim.
#
# Run from the repository root:
#   python3 -I -B research/jumbling/restoration/14-blocking-structure/scripts/peel.py <table.json> <labels.npz>
#       <x> <A> <B> <out.json> [--keep K]
import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import blockmap as BM  # noqa: E402
import jamcheck as J  # noqa: E402

CLS, M, R = BM.CLS, BM.M, BM.R


def phi(h):
    return int(np.maximum(h.astype(np.int64) - 2, 0).sum())


class Tracker:
    """Exact heights, branches and labels at x, updated for the pieces each twist moves."""

    def __init__(self, st, h, b, lx, x):
        self.st, self.x = st, x
        self.h, self.b, self.lx = h.astype(np.int16).copy(), b.astype(np.int8).copy(), lx.copy()

    def inside(self, c):
        return np.flatnonzero(J.labels_at(self.st, np.arange(len(self.h)), c) == 1)

    def apply(self, c, g, moved):
        if not J.twist(self.st, c, g):
            return None
        old = (self.h[moved].copy(), self.b[moved].copy(), self.lx[moved].copy())
        self.h[moved], self.b[moved] = J.tree_rows(self.st, moved)
        self.lx[moved] = J.labels_at(self.st, moved, self.x)
        return old

    def undo(self, c, g, moved, old):
        from exact import transpose
        from sim.kplus import to_tuple
        assert J.twist(self.st, c, to_tuple(transpose(g)))
        self.h[moved], self.b[moved], self.lx[moved] = old

    def cost(self):
        S = int((self.lx == 2).sum())
        F = int(((self.lx == 1) & (self.h >= 2) & (self.b != CLS[self.x])).sum())
        return S, F


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('table')
    ap.add_argument('labels')
    ap.add_argument('x', type=int)
    ap.add_argument('A', type=int)
    ap.add_argument('B', type=int)
    ap.add_argument('out')
    ap.add_argument('--keep', type=int, default=14)
    a = ap.parse_args(argv)
    st = BM.load_table(a.table)
    h, b = BM.tree_info(st)
    lab = BM.labels_for(st, a.labels, 1)
    if (lab[:, a.A] == 2).any():
        raise SystemExit(f'grip {a.A} is blocked')
    tr = Tracker(st, h, b, lab[:, a.x], a.x)
    d0, phi0, n40 = st.digest(), phi(tr.h), int((tr.h == 4).sum())
    t0 = time.time()
    movedA = np.flatnonzero(lab[:, a.A] == 1)
    first = []
    for i in range(1, len(M.items)):
        ga = R.element(M, a.A, i)
        old = tr.apply(a.A, ga, movedA)
        assert old is not None
        S, F = tr.cost()
        first.append((S + F, S, F, i))
        tr.undo(a.A, ga, movedA, old)
    first.sort()
    print(json.dumps(dict(event='first', best=first[:a.keep], sec=round(time.time() - t0))), flush=True)
    rows = []
    for _, _, _, i in first[:a.keep]:
        ga = R.element(M, a.A, i)
        oldA = tr.apply(a.A, ga, movedA)
        movedB = tr.inside(a.B)
        for j in range(1, len(M.items)):
            gb = R.element(M, a.B, j)
            oldB = tr.apply(a.B, gb, movedB)
            if oldB is None:
                rows.append(dict(a=i, b=j, applied=False))
                continue
            S, F = tr.cost()
            row = dict(a=i, b=j, applied=True, S=S, F=F, phi=phi(tr.h), off=int(st.off_lattice_count()))
            if S == 0:
                movedX = tr.inside(a.x)
                best = None
                for k in range(1, len(M.items)):
                    gx = R.element(M, a.x, k)
                    oldX = tr.apply(a.x, gx, movedX)
                    assert oldX is not None
                    v = (phi(tr.h) - phi0, int((tr.h == 4).sum()) - n40, int(tr.h.max()), k)
                    best = v if best is None or v < best else best
                    tr.undo(a.x, gx, movedX, oldX)
                row['lower'] = dict(dphi=best[0], dn4=best[1], max_h=best[2], item=best[3])
            rows.append(row)
            tr.undo(a.B, gb, movedB, oldB)
        tr.undo(a.A, ga, movedA, oldA)
        done = [r for r in rows if r['a'] == i and r['applied']]
        bestrow = min(done, key=lambda r: (r['S'] + r['F'], r['S'], r['b'])) if done else None
        print(json.dumps(dict(event='a', a=i, best=bestrow, clean_after=sum(1 for r in done if r['S'] == 0),
                              sec=round(time.time() - t0))), flush=True)
    assert st.digest() == d0
    Path(a.out).write_text(json.dumps(dict(digest=d0, x=a.x, A=a.A, B=a.B, keep=a.keep, phi=phi0, n4=n40,
                                           first=first, rows=rows), indent=1))


if __name__ == '__main__':
    main()
