# Left heights of random I_a scrambles at checkpoints (18 October 2026): the scramble of restore.scramble(M, k, seed),
# with the same draws, so the state after j applied twists is the start state of a census run of depth j and seed
# seed; every <checkpoint step> applied twists, one JSON line with the off count and the histogram of left heights
# over the off pieces (exact tree distances, as census_seams.py computes them). No descent runs. Writes only the
# output file (appending). Exact arithmetic slows as the scramble deepens: in the runs of 18 to 19 October, 3000
# twists took 1.7 to 1.9 hours each, three at a time on four shared cores.
# Run from the repository root:
#   python3 -I -B research/jumbling/restoration/11-scramble-census/scripts/heights_k.py <k> <seed> <checkpoint step> <out jsonl>
import json, random, sys, time
from pathlib import Path
import numpy as np
REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures')]
import restore, sim, trees as T

k, seed, step, out = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]), Path(sys.argv[4])
M = restore.menu('I_a')
t0 = time.time()


def heights(st):
    pid, cnt = np.unique(st.pose_id, return_counts=True)
    h = {}
    for p, c in zip(pid.tolist(), cnt.tolist()):
        if st._poses[p].kidx >= 0:
            continue
        d = T.dist(T.factors(st._pose_matrix(p))[0])
        h[d] = h.get(d, 0) + c
    return {str(a): h[a] for a in sorted(h)}


rng = random.Random(seed)
st = sim.State(restore.ctx, menu=M)
J = restore.jumble_items(M)
every = list(range(len(M.items)))
hist = []
peak = 0
with out.open('a') as f:
    while len(hist) < k:
        c = rng.randrange(600)
        i = rng.choice(every if rng.random() < 0.0 else J)
        if st.apply(sim.Twist(restore.ctx, c, restore.element(M, c, i), 'menu', {'menu': M.name})).applied:
            hist.append((c, i))
            st.journal = []
            if len(hist) % step == 0:
                h = heights(st)
                mx = max(int(a) for a in h) if h else 0
                peak = max(peak, mx)
                f.write(json.dumps({'k': len(hist), 'seed': seed, 'off': int(st.off_lattice_count()),
                                    'max_height': mx, 'peak_at_checkpoints': peak, 'heights': h,
                                    'seconds': round(time.time() - t0, 1)}) + '\n')
                f.flush()
