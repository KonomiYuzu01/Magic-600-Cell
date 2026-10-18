# Theory round 22, experiment E2: the first preparation for b06 (the class-3 vertex content of cell {91}). A SAFE-ALL
# word X0 that moves the holes of b06's deficit types out of the cell, so that the type count of R22C-V item 4 holds
# for b06 alone at Z.X0 (counted against the cell's chambers; the full count, with b05 and the whole Bad set, is
# e2_b06_prep2.py's), then the grip-91 letters that land b06 there. Greedy over single SAFE letters on the hole set
# (exact cap permutations of buffer.Alphabet); X0 is then replayed on the state and every claim is checked there: the
# off pieces keep their exact poses and the hole image is the predicted one. Research only, headless; exact geometry.
# Writes <out>/x-word.json, <out>/zx.json (the pose table of Z.X0), <out>/w91-<item>.json for each landing letter and
# <out>/free.json, the other off pieces inside cap 91 (a grip-91 letter moves them; W^-1 returns them).
# Usage (from the repository root): python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/e2_b06_prep1.py
# <pose table> <out dir>
import json
import sys
from pathlib import Path
from types import SimpleNamespace

J = Path('research/jumbling')
sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures')]
import buffer as B  # noqa: E402
import numpy as np  # noqa: E402
from exact import matmul  # noqa: E402

O, CL, restore = B.O, B.CL, B.restore
reg, kp = CL.ctx.regions, CL.ctx.kplus
typ = np.asarray(reg.orbit_of)
UNIT = J / 'restoration/12-low-height/runs/round22/line-a/units/eb5-k40-k00-b06.json'


def deficits(H, C, u):
    h = np.bincount(typ[H], minlength=36)
    inside = np.bincount(typ[np.intersect1d(H, C)], minlength=36)
    return {int(t): int(u[t] + inside[t] - h[t]) for t in range(36) if u[t] + inside[t] > h[t]}


def total(d):
    return sum(d.values())


def landing_items(st, M, U):
    out = []
    cl = st.classify(91, need_inside=True, stop_at_straddle=False)
    lat = st.lattice_flags()
    inside = np.asarray(cl.inside, np.int64)
    foreign = sorted(set(int(p) for p in inside[~lat[inside]]) - set(U.tolist()))
    for i in range(len(M.items)):
        g = restore.element(M, 91, i)
        ku = [kp.index_of_matrix(B.to_tuple(matmul(g, st._pose_matrix(int(st.pose_id[p]))))) for p in U]
        if min(ku) >= 0:
            out.append(i)
    return out, cl.straddle is None, foreign


def main():
    table, out = sys.argv[1], Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    R, M = O.start(table)
    st = R.st
    z, fp = st.digest(), B.off_fingerprint(st)
    U = np.array(json.loads(UNIT.read_text())['pieces'], np.int64)
    assert np.isin(U, B.off_pieces(st)).all(), 'b06 is not off at this state'
    home = CL.data.sig_matrix(np.arange(len(st.pose_id)))
    members, _ = CL.classes()
    C = B.cell_chambers(home, members, 3, [91])
    u = np.bincount(typ[U], minlength=36)
    H = B.hole_set(st)
    d0 = deficits(H, C, u)
    G = B.Alphabet(SimpleNamespace(st=st), M)
    # Greedy: each round, the SAFE letter whose hole image lowers the total deficit most (ties: first letter).
    word, cur = [], np.array(H)
    while total(deficits(cur, C, u)) > 0 and len(word) < 40:
        best, bestd = None, total(deficits(cur, C, u))
        for n, (c, i, k, ys) in enumerate(G.letters):
            xs = G.capx[c]
            m = np.isin(cur, xs)
            if not m.any():
                continue
            nxt = cur.copy()
            nxt[m] = ys[np.searchsorted(xs, cur[m])]
            dn = total(deficits(nxt, C, u))
            if dn < bestd:
                best, bestd, bestnext = n, dn, nxt
        if best is None:
            break
        word.append(best)
        cur = np.sort(bestnext)
    X = G.pairs(word)
    for c, i in X:
        R.apply(int(c), int(i))
    H1 = B.hole_set(st)
    assert np.array_equal(np.sort(H1), np.sort(cur)), 'the hole image differs from the replayed state'
    assert B.off_fingerprint(st) == fp, 'X moved an off piece'
    d1 = deficits(H1, C, u)
    items, unstraddled, foreign = landing_items(st, M, U)
    zx = st.digest()
    Path(out / 'x-word.json').write_text(json.dumps({'word': X, 'input_digest': z, 'output_digest': zx}) + '\n')
    B.export_pose_table(st, out / 'zx.json')
    for i in items:
        Path(out / f'w91-{i}.json').write_text(json.dumps({'word': [[91, i]], 'input_digest': zx}) + '\n')
    Path(out / 'free.json').write_text(json.dumps({'pieces': foreign}) + '\n')
    print(json.dumps({'state': z, 'off': len(H), 'deficits_before': d0, 'X': X, 'deficits_after': d1,
                      'holes_in_cell_before': int(len(np.intersect1d(H, C))),
                      'holes_in_cell_after': int(len(np.intersect1d(H1, C))), 'zx': zx,
                      'landing_items': items, 'cut_91_unstraddled': unstraddled,
                      'foreign_off_inside_cap_91': foreign}), flush=True)


if __name__ == '__main__':
    main()
