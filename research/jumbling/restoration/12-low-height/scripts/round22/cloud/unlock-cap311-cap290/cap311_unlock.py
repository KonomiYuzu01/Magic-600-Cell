# Theory round 22, line (d), round 6, cloud line unlock-cap311-cap290, step 2: the monotone single-letter unlock of
# cap 311 (target tau = 311, u = number of straddlers of cap 311), from the state after R22D-75's seven letters
# (1ff0061f..., u = 182) or from the 3000-twist stop of seed 7302 (015d839e..., u = 107). Built from the draft
# r22d86_cap290.py's functions (copied into unlock_common.py). Repeat: the remaining blockers of 311, the caps holding at
# least one of them whole, and which of those are clean; for each clean holder in cap order, every item 1..59 on its
# own copy of the state, recording u311 (None when the letter is not admissible and certain), whether it keeps H6 (n6
# unchanged and every current height-6 piece at its exact pose) and the highest height; the letter taken is the first
# one (cap order, then item order) that keeps H6, leaves no height above 6 and lowers u311 strictly; the full 59-item
# vector of every cap tried is printed, including the cap whose item is taken. Stop when u311 = 0 or no clean holder
# has such an item (the single-letter U family is exhausted). At u311 = 0: every item of cap 311 on a copy, with the
# n6 drop, the counts, digest, off, N and the pieces at the lattice before the letter that it takes off; the one
# lowering n6 most (ties by item) is played and the end state recorded.
# u after a trial letter is counted as (blockers the letter does not move) + (moved pieces that straddle 311 after it);
# a clean cap moves exactly the pieces it holds whole, so this equals len(E.straddler_set(t, 311)), which is checked on
# every letter taken. The label table is the full in/out/straddle table of every piece at every cap, recomputed after a
# letter only for the pieces the letter moved; it is checked against E.straddler_set at the start and after each letter.
# Research only, headless Linux; exact geometry. It writes <out.json> (the letters, the vectors and the end state).
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 JUMBLING_HOME_CACHE=<file>
#   python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cloud/unlock-cap311-cap290/cap311_unlock.py
#   <dir>/s7302-stop.pkl after75|stop <out.json> [tau]
import sys, json
sys.path.insert(0, 'research/jumbling/restoration/12-low-height/scripts/round22/cloud/unlock-cap311-cap290')
import numpy as np
from unlock_common import (E, load, SEVEN, STOP, AFTER75, clone, put, heights, keys, strad_among, all_labels,
                           snapshot, Labels, emit, cpu)


def scan_cap(s, lab, B, c, tau, six, sixkeys, threshold):
    """The 59 items of clean cap c on copies of s: per item None or [u_tau, keeps_H6, max_height]; and the first item
    that keeps H6, has max height <= 6 and u_tau < threshold, with its state and moved pieces."""
    mv = lab.moved(c); Bset = set(B.tolist()); rest = len(Bset - set(mv.tolist()))
    vec, first = [], None
    for j in range(1, 60):
        t = clone(s)
        if not put(t, c, j):
            vec.append(None); continue
        h = heights(t)
        keep = int((h == 6).sum()) == len(six) and keys(t, six) == sixkeys
        u = rest + len(strad_among(t, mv, tau))
        vec.append([u, keep, int(h.max())])
        if first is None and keep and h.max() <= 6 and u < threshold:
            first = (j, t, u)
    return vec, first, mv


def unlock(s, tau, record, family='U'):
    lab = Labels(s)
    assert lab.cnt[tau] == len(E.straddler_set(s, tau))
    h = heights(s); six = np.flatnonzero(h == 6); sixkeys = keys(s, six)
    emit('START', tau=tau, u=int(lab.cnt[tau]), clean_caps=int((lab.cnt == 0).sum()), cpu=cpu(), **snapshot(s, h))
    while lab.cnt[tau] > 0:
        B = np.flatnonzero(lab.L[:, tau] == 2); u0 = len(B)
        holders = [int(c) for c in np.flatnonzero((lab.L[B] == 1).any(axis=0))]
        clean = [c for c in holders if lab.cnt[c] == 0]
        emit('HOLDERS', u=u0, holders=holders, clean=clean, blocked_u={c: int(lab.cnt[c]) for c in holders if lab.cnt[c]},
             whole_in_clean={c: int((lab.L[B, c] == 1).sum()) for c in clean})
        taken = None
        for c in clean:
            vec, first, mv = scan_cap(s, lab, B, c, tau, six, sixkeys, u0)
            emit('ITEMS', cap=c, u_before=u0, items_1_to_59=vec, cpu=cpu())
            record['vectors'].append(dict(u_before=u0, cap=c, items=vec))
            if first is not None:
                taken = (c, first, mv); break
        if taken is None:
            emit('EXHAUSTED', u=u0, tried_clean_holders=clean, cpu=cpu())
            record['stall'] = dict(u=u0, blockers=B.tolist(), holders=holders, clean=clean,
                                   blocked_u={c: int(lab.cnt[c]) for c in holders if lab.cnt[c]}, state=snapshot(s))
            return s, lab, False
        c, (j, t, u), mv = taken
        assert np.isin(np.flatnonzero(E.stable_pose(s) != E.stable_pose(t)), mv).all()
        s = t; lab = lab.commit(mv, all_labels(s, mv))
        assert lab.cnt[tau] == u == len(E.straddler_set(s, tau))
        snap = snapshot(s); assert snap['max_height'] <= 6
        emit('STEP', letter=[c, j], family=family, u=u, cpu=cpu(), **snap)
        record['letters'].append(dict(letter=[c, j], family=family, u=u, **snap))
    return s, lab, True


def best_h6(s, tau, record):
    h0 = heights(s); n6 = int((h0 == 6).sum()); rows = []
    for j in range(1, 60):
        t = clone(s)
        if not put(t, tau, j):
            rows.append(dict(item=j, applied=False)); continue
        h = heights(t)
        rows.append(dict(item=j, applied=True, n6_drop=n6 - int((h == 6).sum()), lattice_taken_off=int(((h0 == 0) & (h > 0)).sum()),
                         **snapshot(t, h)))
    for r in rows:
        emit('H6_ITEM', cap=tau, **r)
    record['h6_items'] = rows
    ok = [r for r in rows if r['applied'] and r['max_height'] <= 6 and r['n6_drop'] > 0]
    if not ok:
        emit('H6_NONE', cap=tau); return s
    r = max(ok, key=lambda r: (r['n6_drop'], -r['item']))
    assert put(s, tau, r['item']) and s.digest() == r['digest']
    record['letters'].append(dict(letter=[tau, r['item']], family='H6', u=0, **snapshot(s)))
    emit('H6_PLAYED', letter=[tau, r['item']], n6_drop=r['n6_drop'], cpu=cpu(), **snapshot(s))
    return s


def main():
    s, _ = load(sys.argv[1], E)
    start, out = sys.argv[2], sys.argv[3]
    tau = int(sys.argv[4]) if len(sys.argv) > 4 else 311
    if start == 'after75':
        for c, j in SEVEN:
            assert put(s, c, j)
        assert s.digest() == AFTER75
    else:
        assert start == 'stop' and s.digest() == STOP
    record = dict(start=start, start_digest=s.digest(), tau=tau, letters=[], vectors=[])
    s, lab, done = unlock(s, tau, record)
    if done:
        s = best_h6(s, tau, record)
    record['end'] = snapshot(s); record['cpu'] = cpu()
    json.dump(record, open(out, 'w'), indent=1, default=int)
    emit('END', done=done, cpu=cpu(), **record['end'])


main()
