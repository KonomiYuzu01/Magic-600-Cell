# Theory round 22, line (d), round 6, cloud line unlock-cap311-cap290: builds only <dir>/s7302-stop.pkl (the
# 3000-twist stop of seed 7302, digest 015d839e..., 100,584 off), with the same replay, checks and pickle layout as
# r22d31_snapshots.py's main() (its SNAPSHOTS entry and load() are imported, not changed); r22d31_snapshots.py also
# builds the 7103 snapshot, which this line does not need. About 30 CPU minutes.
# Research only, headless Linux; exact geometry. It writes <dir>/s7302-stop.pkl and nothing else.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 JUMBLING_HOME_CACHE=<file>
#   python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cloud/unlock-cap311-cap290/build_stop.py <dir>
import sys, json, pickle
from pathlib import Path
sys.path[:0] = ['research/jumbling/restoration/12-low-height/scripts', 'research/jumbling/restoration/12-low-height/scripts/round22',
                'research/jumbling/theory', 'research/jumbling', 'research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import SNAPSHOTS, load

out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
C = E.sim.get_context(); M = E.R.menu('I_a')
name, rec, upto, digest, off = SNAPSHOTS[1]
assert name == 's7302-stop.pkl'
r = json.loads(Path(rec).read_text())
st = E.sim.State(C, filtered=False, menu=M)
moves = r['moves'] if upto is None else r['moves'][:upto]
for c, j in r['history'] + moves:
    o = st.apply(E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain
assert st.digest() == digest and int(st.off_lattice_count()) == off
pickle.dump({'record': rec, 'history': len(r['history']), 'moves': len(moves), 'digest': digest, 'off': off,
             'pose_id': st.pose_id.copy(), 'poses': [(p.kidx, p.matrix, p.key) for p in st._poses]},
            open(out / name, 'wb'), protocol=5)
load(out / name, E)
print(name, rec, len(r['history']), len(moves), digest, off, flush=True)
