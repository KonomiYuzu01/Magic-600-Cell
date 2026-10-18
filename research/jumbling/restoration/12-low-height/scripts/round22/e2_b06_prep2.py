# Theory round 22, experiment E2: the second preparation for b06 and b05, which one grip-91 letter W lands together.
# W's element is off K+, so T = Bad(W^-1) at Z1 = Z.X.W is the whole chamber set of cap 91, and the chambers of T that
# are occupied at Z1 are the landing chambers L of the two units (fixed, since a SAFE-ALL X keeps the off poses). N2 at
# Z1 for an orbit o of its SAFE group reads |holes(Z.X) outside T in o| >= |L in o|. The orbits are read from a
# reference middle Z1 = Z.X0.W whose N2 failed (written by e2_b06_t1.py for the first preparation X0). X0 is replayed
# at Z, then a greedy over single SAFE letters: phase 1 keeps the holes inside T (so the off geometry of the middle,
# hence its orbits, stays the reference one), phase 2 may move holes out of T (each adds a lattice piece to Q, so the
# result is re-checked at the actual middle by tclose2 in e2_b06_t1.py). Every claim is checked on the replayed state:
# the hole image, the off poses, the remaining deficit. Research only, headless; exact geometry.
# Writes <out>/x-word.json (X0 followed by the new letters, from Z), <out>/zx.json (the pose table of Z.X) and
# <out>/free.json (the off pieces inside cap 91 other than b06, which W turns and W^-1 returns).
# Usage (from the repository root): python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/e2_b06_prep2.py
# <Z table> <X0 word json> <Z1 reference table> <target json> <out dir>
import hashlib
import json
import sys
from pathlib import Path
from types import SimpleNamespace

J = Path('research/jumbling')
sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures')]
import buffer as B  # noqa: E402
import numpy as np  # noqa: E402

O = B.O
UNIT = J / 'restoration/12-low-height/runs/round22/line-a/units/eb5-k40-k00-b06.json'


def main():
    zt, x0f, z1t, tf, out = sys.argv[1], sys.argv[2], sys.argv[3], Path(sys.argv[4]), Path(sys.argv[5])
    out.mkdir(parents=True, exist_ok=True)
    tgt = json.loads(tf.read_text())
    T = np.asarray(tgt['T'], np.int64)
    assert hashlib.sha256(json.dumps(T.tolist()).encode()).hexdigest() == tgt['T_digest']
    R1, M1 = O.start(z1t)
    G1 = B.Alphabet(SimpleNamespace(st=R1.st), M1)
    G1.orbits()
    assert tgt['state_digest'] == R1.st.digest(), 'the target is not the reference middle\'s'
    lab, size = np.asarray(G1.orbit), np.asarray(G1.orbit_size)
    L = T[G1.occupied[T]]
    inT = np.zeros(G1.NP, bool)
    inT[T] = True
    assert np.array_equal(np.sort(T), np.flatnonzero(G1.home[:, 91])), 'T is not the chamber set of cap 91'
    assert (size[lab[L]] > 1).all(), 'a landing chamber is fixed by every SAFE letter (N1 fails)'
    need = np.bincount(lab[L], minlength=len(size))
    orbs = np.flatnonzero(need)
    print(json.dumps({'reference_middle': R1.st.digest(), 'T': len(T), 'L': len(L)}), flush=True)
    del R1, G1

    def deficit(H):
        have = np.bincount(lab[H[~inT[H]]], minlength=len(size))
        d = need[orbs] - have[orbs]
        return int(d[d > 0].sum()), {int(o): int(x) for o, x in zip(orbs, d) if x > 0}
    x0 = json.loads(Path(x0f).read_text())
    R, M = O.start(zt, [x0f])
    st = R.st
    z0, fp = st.digest(), B.off_fingerprint(st)
    U = np.array(json.loads(UNIT.read_text())['pieces'], np.int64)
    assert np.isin(U, B.off_pieces(st)).all(), 'b06 is not off at this state'
    G = B.Alphabet(SimpleNamespace(st=st), M)
    H0 = np.sort(B.hole_set(st))
    H0in = H0[inT[H0]]
    print(json.dumps({'start': x0['input_digest'], 'after_X0': z0, 'holes': len(H0), 'holes_in_T': len(H0in),
                      'deficit': deficit(H0)}), flush=True)
    word, cur = [], H0.copy()
    for phase in (1, 2):
        while deficit(cur)[0] > 0 and len(word) < 60:
            best, bestd = None, deficit(cur)[0]
            for n, (c, i, k, ys) in enumerate(G.letters):
                xs = G.capx[c]
                m = np.isin(cur, xs)
                if not m.any():
                    continue
                nxt = cur.copy()
                nxt[m] = ys[np.searchsorted(xs, cur[m])]
                nxt.sort()
                if phase == 1 and not np.array_equal(nxt[inT[nxt]], H0in):
                    continue
                dn = deficit(nxt)[0]
                if dn < bestd:
                    best, bestd, bestnext = n, dn, nxt
            if best is None:
                break
            word.append(best)
            cur = bestnext
            print(json.dumps({'phase': phase, 'letter': G.pairs([best])[0], 'deficit': deficit(cur)}), flush=True)
    X = G.pairs(word)
    for c, i in X:
        R.apply(int(c), int(i))
    H1 = np.sort(B.hole_set(st))
    assert np.array_equal(H1, cur), 'the hole image differs from the replayed state'
    assert B.off_fingerprint(st) == fp, 'X moved an off piece'
    zx = st.digest()
    cl = st.classify(91, need_inside=True, stop_at_straddle=False)
    inside = np.asarray(cl.inside, np.int64)
    free = sorted(set(int(p) for p in inside[~st.lattice_flags()[inside]]) - set(U.tolist()))
    Path(out / 'x-word.json').write_text(json.dumps({'word': x0['word'] + X, 'input_digest': x0['input_digest'],
                                                     'output_digest': zx}) + '\n')
    B.export_pose_table(st, out / 'zx.json')
    Path(out / 'free.json').write_text(json.dumps({'pieces': free}) + '\n')
    print(json.dumps({'X0': x0['word'], 'new': X, 'deficit_after': deficit(H1), 'holes_in_T_after': int(inT[H1].sum()),
                      'zx': zx, 'free_inside_cap_91': len(free), 'cut_91_unstraddled': cl.straddle is None}), flush=True)


if __name__ == '__main__':
    main()
