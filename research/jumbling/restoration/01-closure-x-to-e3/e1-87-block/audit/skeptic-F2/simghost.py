"""Independent recomputation of Bad(W), Bad(W^-1) and T's landing with the simulator's own State.apply.

No ghostlib / fastghost: every twist is applied by sim.State.apply (the admissibility ground truth). When a twist is
blocked, every straddling piece is found with the simulator's own exact classifier (same logic as State.classify),
recorded as dead at that step and moved to a lattice slot of its orbit outside the twisting cap; then the twist is
retried and must be admissible. A content is Bad if it died or ended off K+.

Runs (for candidate n of cands-87.json, W = P Q P^-1):
  A: solved state, word W     -> Bad_A(W) over all chambers
  B: solved state, word W^-1  -> Bad_B(W^-1)
  C: E1 state, word W         -> T on K, landing L (real T pieces), B' exact poses, E1 lattice pieces that die/end off
Usage: python3 -I simghost.py <n> [runs, default ABC]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
S = Path('<work>')
TH = S / 'wt-e3/research/jumbling/theory'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import blocks as BL  # noqa: E402
import sim  # noqa: E402
from exact import identity, transpose  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

O = BL.O
ctx = BL.CL.ctx
data, kp, reg = ctx.data, ctx.kplus, ctx.regions
NP = len(data.sigbits)
IDK = kp.index_of_matrix(to_tuple(identity(4)))
OUT = S / 'v26/audit/skeptic-F2'
E1 = S / 'v26/E2/c0/E1-table.json'
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), flush=True)


def slot_of(p, k):
    p = np.asarray(p, np.int64)
    k = np.asarray(k, np.int64)
    return reg.piece_of[reg.orbit_of[p], kp.compose_vec(k, reg.transport[p])]


def cap(c):
    return data.in_sig(np.arange(NP), np.full(NP, int(c), np.int64))


_OUTSLOT = {}


def out_t(o, c):
    """A K+ element t with piece_of[o, t] outside cap c."""
    key = (int(o), int(c))
    if key not in _OUTSLOT:
        sl = reg.piece_of[o]
        ins = data.in_sig(sl, np.full(len(sl), int(c), np.int64))
        t = int(np.flatnonzero(~ins)[0])
        _OUTSLOT[key] = t
    return _OUTSLOT[key]


def park(st, pieces, c):
    """Move pieces to lattice poses at slots of their orbit outside cap c."""
    for p in pieces:
        o, tp = int(reg.orbit_of[p]), int(reg.transport[p])
        k = kp.compose(out_t(o, c), int(kp.inv[tp]))
        s = int(slot_of([p], [k])[0])
        assert not data.in_sig(np.array([s]), np.array([int(c)]))[0]
        st.pose_id[p] = st._intern_k(k)
    st._compact()
    st._anchor_cache = {}


def all_straddlers(st, e):
    """Every piece straddling the cut of pole e (lattice pieces never straddle)."""
    K = st._kpose()[st.pose_id]
    off = np.nonzero(K < 0)[0]
    out = []
    for pid, members in (st._groups(off) if off.size else []):
        ev = Evaluator(pullback_form(st._pose_matrix(pid), data.N[e]))
        for anchor, sub in st._anchors(pid, members):
            r = ev.classify(reg.cap_vform(anchor))
            if r[0] == 'uncertain':
                raise RuntimeError('uncertain')
            if r[0] != 'straddle':
                continue
            for p in sub.tolist():
                r2 = ev.classify(reg.vform(p))
                if r2[0] == 'straddle':
                    out.append(int(p))
    return out


def run(st, R, M, word, label):
    dead = {}
    blocked_steps = 0
    for step, (c, i) in enumerate(word):
        if dead:
            dp = np.array(sorted(dead), np.int64)
            K = st._kpose()[st.pose_id[dp]]
            assert (K >= 0).all()
            sl = slot_of(dp, K)
            ins = dp[data.in_sig(sl, np.full(len(sl), int(c), np.int64))]
            if len(ins):
                park(st, ins.tolist(), c)
        tw = sim.Twist(ctx, int(c), R.g(c, i), 'menu', {'menu': M.name})
        tries = 0
        while True:
            out = st.apply(tw)
            if out.applied:
                break
            if out.status != 'blocked':
                raise RuntimeError(f'{label} step {step}: {out.status}')
            strs = all_straddlers(st, int(c))
            if not strs:
                raise RuntimeError(f'{label} step {step}: blocked but no straddler found')
            for p in strs:
                dead[p] = step
            park(st, strs, c)
            tries += 1
            blocked_steps += 1
            if tries > 3:
                raise RuntimeError('too many tries')
        log(run=label, step=step, letter=[int(c), int(i)], moved=int(out.moved), dead=len(dead),
            off=st.off_lattice_count())
    return dead, blocked_steps


def inverse_word(R, M, w):
    n_items = len(M.items)
    out = []
    for c, i in reversed(w):
        t = to_tuple(transpose(R.g(c, i)))
        out.append((c, next(j for j in range(n_items) if to_tuple(R.g(c, j)) == t)))
    return out


def solved(R):
    st = R.st
    st.pose_id[:] = st._intern_k(IDK)
    st._compact()
    st.journal, st._anchor_cache = [], {}
    assert st.off_lattice_count() == 0
    return st


def bad_from_solved(word, label):
    R, M = O.start(str(E1))
    st = solved(R)
    grips = sorted({c for c, _ in word})
    U = np.flatnonzero(np.logical_or.reduce([cap(c) for c in grips]))
    dead, nb = run(st, R, M, word, label)
    K = st._kpose()[st.pose_id]
    deadarr = np.zeros(NP, bool)
    deadarr[list(dead)] = True
    badmask = deadarr | (K < 0)
    bad = np.flatnonzero(badmask)
    good = np.flatnonzero(~badmask)
    ends = slot_of(good, K[good])
    moved_outside_U = int((~np.isin(good, U) & (ends != good)).sum())
    return {'U': int(len(U)), 'bad': bad.tolist(), 'n_bad': int(len(bad)), 'bad_outside_U': int((~np.isin(bad, U)).sum()),
            'good_end_injective': bool(len(np.unique(ends)) == len(ends)), 'moved_outside_U': moved_outside_U,
            'died': int(len(dead)), 'ended_off': int(((K < 0) & ~deadarr).sum()), 'blocked_steps': nb,
            'good_ends': dict(zip(good[np.isin(good, U)].tolist(), ends[np.isin(good, U)].tolist()))}


def main():
    n = int(sys.argv[1])
    runs = sys.argv[2] if len(sys.argv) > 2 else 'ABC'
    cand = json.loads((S / 'v26/E2/cands-87.json').read_text())[n]
    R0, M0 = O.start(str(E1))
    P = [tuple(x) for x in cand['P']]
    Q = [tuple(x) for x in cand['Q']]
    W = P + Q + inverse_word(R0, M0, P)
    Wi = inverse_word(R0, M0, W)
    res = {'n': n, 'source': cand['source'], 'W': [list(x) for x in W]}
    log(n=n, W=len(W))
    if 'C' in runs:
        R, M = O.start(str(E1))
        st = R.st
        dg = st.digest()
        K0 = st._kpose()[st.pose_id]
        off = np.flatnonzero(K0 < 0)
        bl = BL.Blocks(R, M)
        T = np.asarray(next(b for b in bl.blocks if len(b) == 87), np.int64)
        Bp = np.setdiff1d(off, T)
        Bp_pose0 = {int(p): st.pose(int(p)) for p in Bp}
        lat = np.flatnonzero(K0 >= 0)
        slot0 = np.full(NP, -1, np.int64)
        slot0[lat] = slot_of(lat, K0[lat])
        occ = np.zeros(NP, bool)
        occ[slot0[lat]] = True
        dead, nb = run(st, R, M, W, 'C')
        K = st._kpose()[st.pose_id]
        T_dead = [int(p) for p in T if int(p) in dead]
        Bp_dead = [int(p) for p in Bp if int(p) in dead]
        T_on = int((K[T] >= 0).sum()) if not T_dead else None
        L = slot_of(T, K[T]).tolist() if T_on == 87 else None
        lat_dead = [p for p in dead if K0[p] >= 0]
        lat_off_end = [int(p) for p in lat if K[p] < 0 and int(p) not in dead]
        res['C'] = {'E1_digest': dg, 'T': len(T), 'Bp': Bp.tolist(), 'T_dead': T_dead, 'Bp_dead': Bp_dead,
                    'T_on': T_on, 'L': L,
                    'Bp_exact': all(st.pose(int(p)) == Bp_pose0[int(p)] for p in Bp) if not Bp_dead else False,
                    'lattice_dead': len(lat_dead), 'lattice_end_off': len(lat_off_end),
                    'bad_slots_occupied_E1': sorted(int(slot0[p]) for p in lat_dead + lat_off_end),
                    'holes_E1': np.flatnonzero(~occ).tolist(), 'blocked_steps': nb}
        log(C={k: v for k, v in res['C'].items() if k not in ('L', 'bad_slots_occupied_E1', 'holes_E1')})
    if 'A' in runs:
        res['A'] = bad_from_solved(W, 'A')
        log(A={k: v for k, v in res['A'].items() if k not in ('bad', 'good_ends')})
    if 'B' in runs:
        res['B'] = bad_from_solved(Wi, 'B')
        log(B={k: v for k, v in res['B'].items() if k not in ('bad', 'good_ends')})
    for k in ('A', 'B'):
        if k in res:
            res[k].pop('good_ends')
    (OUT / f'simghost-{n}-{runs}.json').write_text(json.dumps(res) + '\n')


if __name__ == '__main__':
    main()
