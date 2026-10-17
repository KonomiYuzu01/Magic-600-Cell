# Scramble census for the tutorial line (17 October 2026): phase U (rim descent, research/jumbling/theory/seams.py,
# default rule order) on a fresh random I_a scramble, with a wall-clock budget. Reads only the configuration; the
# scramble history is stored for labelling and never used by the rules. Writes only into the output directory.
# Run from the repository root:
#   python3 -I -B census_seams.py <k> <seed> <budget seconds> <out dir> [--start <previous result json>]
import sys, json, time, os
from pathlib import Path
import numpy as np
REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures')]
import seams, restore, sim, trees as T

k, seed, budget, out = int(sys.argv[1]), int(sys.argv[2]), float(sys.argv[3]), Path(sys.argv[4])
start = None
if '--start' in sys.argv:
    start = json.loads(Path(sys.argv[sys.argv.index('--start') + 1]).read_text())
out.mkdir(parents=True, exist_ok=True)
tag = f'ia-k{k}-s{seed}'
dest = out / f'{tag}.json'
t0 = time.time()
M = restore.menu('I_a')
st, hist = restore.scramble(M, k, seed)
off0 = int(st.off_lattice_count())


def heights(st):
    """Histogram of left heights over the off pieces (exact tree distances, one per off pose)."""
    pid, cnt = np.unique(st.pose_id, return_counts=True)
    h = {}
    for p, c in zip(pid.tolist(), cnt.tolist()):
        if st._poses[p].kidx >= 0:
            continue
        d = T.dist(T.factors(st._pose_matrix(p))[0])
        h[d] = h.get(d, 0) + c
    return {str(a): h[a] for a in sorted(h)}, sum(1 for p in pid.tolist() if st._poses[p].kidx < 0)


R = seams.SeamRestorer(st, M)
prior = start['moves'] if start else []
for c, i in prior:
    R.apply(c, i)
N0 = R.misalignment()
h0, poses0 = heights(st)
info = {'run': tag, 'menu': 'I_a', 'k': k, 'seed': seed, 'history': hist, 'off_scramble': off0,
        'continued_from_moves': len(prior), 'N_start': N0, 'off_start': int(st.off_lattice_count()),
        'heights_start': h0, 'off_poses_start': poses0, 'order': list(R.order), 'budget_s': budget}
print(json.dumps({x: info[x] for x in info if x != 'history'}), flush=True)
rules = {'seam': R.seam, 'cover': R.cover, 'close': R.level1, 'lock': R.level2,
         'improve': R.improve, 'shift': R.shift, 'conj': R.conj, 'reopen': R.reopen}
steps = list(start['steps']) if start else []
t1 = time.time()


def save(result):
    rec = dict(info)
    rec.update({'result': result, 'moves': R.moves, 'steps': steps, 'twists': len(R.moves),
                'N_now': R.misalignment(), 'off_now': int(st.off_lattice_count()),
                'grip_N_now': int(R.grip_misalignment()), 'misaligned_grips_now': len(R.survey()),
                'seconds': round(time.time() - t0, 1)})
    tmp = dest.with_suffix('.tmp')
    tmp.write_text(json.dumps(rec) + '\n')
    os.replace(tmp, dest)
    return rec


result = None
while True:
    N = R.misalignment()
    if N == 0:
        result = 'lattice'
        break
    if time.time() - t1 > budget:
        result = 'time-limit'
        break
    done = rule = None
    for rule in R.order:
        done = rules[rule]()
        if done is not None:
            break
    if done is None:
        result = 'stuck'
        break
    steps.append({'rule': rule, 'twists': done if isinstance(done, int) else len(done) if hasattr(done, '__len__') else done,
                  'N_before': N, 'N_after': R.misalignment(), 'off_after': int(st.off_lattice_count()),
                  'sec': round(time.time() - t1)})
    print(json.dumps(steps[-1]), flush=True)
    save('running')
rec = save(result)
hE, posesE = heights(st)
rec.update({'heights_end': hE, 'off_poses_end': posesE})
if result == 'lattice':
    v, vin = restore.end_invariant(st)
    rec.update({'v_end_in_vG': bool(vin),
                'solved': bool(np.array_equal(st.lattice_stickers()['labels'], np.arange(restore.NS)))})
tmp = dest.with_suffix('.tmp'); tmp.write_text(json.dumps(rec) + '\n'); os.replace(tmp, dest)
print(json.dumps({x: rec[x] for x in ('run', 'result', 'twists', 'N_start', 'N_now', 'off_start', 'off_now',
                                      'heights_end', 'seconds')}), flush=True)
