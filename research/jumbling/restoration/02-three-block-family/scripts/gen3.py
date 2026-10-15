"""Freeze new test inputs reachable from E2, with the B' block back at its exact E2 pose (fixed-pose family).

Each input is E2 followed by a generating word applied on the exact simulator (every twist checked admissible):
  safe   <seed> <n>          n seeded random SAFE-ALL letters of the E2 off geometry (they never move B').
  setup  <seed> <n> <seed2> <n2>
                             V . W* . kappa . W*^-1 . r, where V is the clearing word for Bad(W*) at E2, W* the
                             segment-3 template word, kappa n seeded random K letters at the all-on-K state on grips
                             whose caps miss B''s landing chambers (B' stays put), W*^-1 returns B' to its E2 pose and
                             re-opens the three holes, and r is n2 seeded random SAFE-ALL letters. The setup changes
                             the occupancy by a chamber permutation outside the SAFE-ALL group's reach in general.
The script writes the pose table (input) and, separately, the generation record (word, seeds, digests: the
reachability certificate). A constructor is given the table only.

Usage: python -I gen3.py <theory dir> <v26 dir> <E2 table> <template json> <out prefix> safe <seed> <n>
       python -I gen3.py <theory dir> <v26 dir> <E2 table> <template json> <out prefix> setup <seed> <n> <seed2> <n2>
"""
import json
import random
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import layers3 as L3  # noqa: E402

B, CP, GL = L3.B, L3.CP, L3.GL


def main():
    a = sys.argv[1:]
    table, tfile, prefix, mode = a[2], a[3], a[4], a[5]
    sigma = json.loads(Path(tfile).read_text())
    C = GL.Ctx(table)
    R, M, st = C.R, C.M, C.st
    d0 = st.digest()
    poses0 = {int(p): L3.matrix_json(st.pose(int(p))) for p in C.off}
    G = B.Alphabet(R, M)
    word, record = [], {'source_table_digest': d0, 'mode': mode}
    if mode == 'safe':
        seed, n = int(a[6]), int(a[7])
        rng = random.Random(seed)
        word = [tuple(G.pairs([rng.randrange(len(G.letters))])[0]) for _ in range(n)]
        record.update(seed=seed, n=n, parts={'r': len(word)})
    elif mode == 'setup':
        seed, n, seed2, n2 = int(a[6]), int(a[7]), int(a[8]), int(a[9])
        cert = L3.certify(C, sigma, log_rows=False)
        if not cert['template_ok']:
            raise SystemExit('template conditions fail at the source')
        D = cert['finish']['D_predicted']
        vac, V, _ = L3.vacancy(C, D, jordan=False, G=G)
        if V is None:
            raise SystemExit('no clearing word at the source')
        W = [tuple(x) for x in cert['W']]
        land = cert['finish']['landing_predicted']
        H = L3.HOME()
        free = [c for c in range(600) if not H[land, c].any()]
        rng = random.Random(seed)
        kl = L3.kplus_letters(C, free)
        kappa = [kl[rng.randrange(len(kl))] for _ in range(n)]
        rng2 = random.Random(seed2)
        r = [tuple(G.pairs([rng2.randrange(len(G.letters))])[0]) for _ in range(n2)]
        word = V + W + kappa + CP.inverse_word(M, W) + r
        record.update(seed=seed, n=n, seed2=seed2, n2=n2, D=D, landing=land,
                      parts={'V': len(V), 'W': len(W), 'kappa': len(kappa), 'W_inverse': len(W), 'r': len(r)})
    else:
        raise SystemExit(f'unknown mode {mode}')
    for c, i in word:
        R.apply(int(c), int(i))
    off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
    if off.tolist() != C.off.tolist():
        raise AssertionError('the off set changed')
    same = all(L3.matrix_json(st.pose(int(p))) == poses0[int(p)] for p in off.tolist())
    if not same:
        raise AssertionError("B' is not at its source pose")
    out_table = Path(prefix + '-table.json')
    B.export_pose_table(st, out_table)
    holes = B.hole_set(st)
    typ = np.asarray(L3.reg.orbit_of)
    G.orbits()
    record.update(word=[list(map(int, x)) for x in word], length=len(word), output_digest=st.digest(),
                  table=out_table.name, holes=holes.tolist(), hole_types=typ[holes].tolist(),
                  hole_orbits_E2_alphabet=G.orbit[holes].tolist(), off=off.tolist(), block_at_source_pose=same)
    Path(prefix + '-generation.json').write_text(json.dumps(record, indent=1) + '\n')
    L3.log(done=prefix, length=len(word), digest=st.digest(), holes=holes.tolist(),
           orbits=G.orbit[holes].tolist())


if __name__ == '__main__':
    main()
