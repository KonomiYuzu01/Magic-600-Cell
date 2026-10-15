"""Audit 'shrink': independent word-only chamber ghost, Bad(W) for W = P(k)^-1 (and optionally P(k)).

Own implementation (no fastghost / ghostlib / conjclose): a fresh simulator state with every piece z at the identity
stands for the lattice content of chamber z (its region is chamber z). For each twist (c, h) of W the simulator's
exact classifier gives the inside set; if it reports a straddle, every alive off content that is not inside is
classified individually with the exact kernel and the straddlers die (frozen at the identity, never moved again).
Alive inside contents get h on the left (State._commit, no admissibility test: contents are independent).
Bad(W) = dead contents and contents that end at a pose outside K+.

Usage: python3 -I -B myghost.py <out json> <levels...>
"""
import json
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
S = Path('<work>')
TH = S / 'wt-e3/research/jumbling/theory'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))

import numpy as np  # noqa: E402

import blocks as BL  # noqa: E402
import sim  # noqa: E402
from exact import matmul, transpose  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

O, CL, restore = BL.O, BL.CL, BL.restore
ctx = CL.ctx
kp, reg, data = ctx.kplus, ctx.regions, ctx.data
XS = [(91, 6), (154, 7), (133, 1), (183, 6)]
T0 = time.time()


def log(*a):
    print(round(time.time() - T0, 1), *a, file=sys.stderr, flush=True)


def inv_letter(M, c, i):
    e = restore.element(M, c, i)
    et = to_tuple(transpose(e))
    hits = [j for j in range(len(M.items)) if restore.element(M, c, j) == et]
    assert len(hits) == 1
    assert to_tuple(matmul(e, restore.element(M, c, hits[0]))) == restore.I4
    return (c, hits[0])


def inv_word(M, w):
    return [inv_letter(M, c, i) for c, i in reversed(w)]


def ghost(M, word):
    g = sim.State(ctx)
    NP = len(g.pose_id)
    alive = np.ones(NP, bool)
    died = np.full(NP, -1, np.int64)
    nstrad = 0
    for j, (c, i) in enumerate(word):
        cl = g.classify(int(c), need_inside=True, stop_at_straddle=False)
        if cl.uncertain:
            raise AssertionError('uncertain classification in ghost')
        ins = np.asarray(cl.inside, np.int64)
        if cl.straddle is not None:
            K = g._kpose()[g.pose_id]
            isin = np.zeros(NP, bool)
            isin[ins] = True
            cand = np.flatnonzero(alive & (K < 0) & ~isin)
            evs = {}
            dead = []
            for p in cand.tolist():
                pid = int(g.pose_id[p])
                ev = evs.get(pid)
                if ev is None:
                    ev = evs[pid] = Evaluator(pullback_form(g._pose_matrix(pid), data.N[int(c)]))
                if ev.classify(reg.vform(p))[0] == 'straddle':
                    dead.append(p)
            dead = np.array(dead, np.int64)
            if len(dead):
                alive[dead] = False
                died[dead] = j
                g.pose_id[dead] = 0
                nstrad += len(dead)
        ins = ins[alive[ins]]
        tw = sim.Twist(ctx, int(c), restore.element(M, c, i), 'menu', {'menu': M.name})
        g._commit(tw, ins, journal=False)
    K = g._kpose()[g.pose_id]
    bad = ~alive | (K < 0)
    return bad, alive, died, nstrad


def main():
    out = Path(sys.argv[1])
    levels = [int(x) for x in sys.argv[2:]]
    R, M = O.start(str(S / 'v26/B4/E2-table.json'))
    P = [[(237, 13)]]
    for k in range(4):
        P.append(P[k] + [XS[k]] + inv_word(M, P[k]))
    res = {}
    for k in levels:
        t = time.time()
        bad, alive, died, nstrad = ghost(M, inv_word(M, P[k]))
        badf, alivef, _, nstradf = ghost(M, P[k])
        res[k] = {'Bad_Pinv': np.flatnonzero(bad).tolist(), 'dead_Pinv': np.flatnonzero(~alive).tolist(),
                  'Bad_P': np.flatnonzero(badf).tolist(), 'dead_P': np.flatnonzero(~alivef).tolist(),
                  'sec': round(time.time() - t, 1)}
        log('level', k, 'Bad(P^-1)', int(bad.sum()), 'dead', int((~alive).sum()), 'Bad(P)', int(badf.sum()),
            'dead(P)', int((~alivef).sum()))
    out.write_text(json.dumps(res) + '\n')


if __name__ == '__main__':
    main()
