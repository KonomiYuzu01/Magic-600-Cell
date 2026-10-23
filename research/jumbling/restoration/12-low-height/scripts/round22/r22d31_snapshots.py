# Theory round 22, line (d), round 4 (R22D-31 to R22D-38): the two start states of the height-6 questions, saved as
# snapshots so that r22d31_height6.py does not replay the records each time. Each state is rebuilt from its census
# record (the history, then the first descent twists), every twist admissible, and saved with pickle as its pose ids
# and pose table (K+ index, exact matrix, key), its digest and its off count:
#   s7103-before74.pkl  seed 7103, depth 1000 (runs/round2/ia-k1000-s7103.json), the history and the first 187 descent
#                       twists: the state before step 74 (R22D-30), digest 48ef2f68..., 94,981 off pieces;
#   s7302-stop.pkl      seed 7302, depth 3000 (runs/heights/ia-k3000-s7302.json), the history and all 313 descent
#                       twists: the stop, digest 015d839e..., 100,584 off pieces.
# load(path, E) rebuilds a sim.State from a snapshot and checks its digest and off count. Open only snapshots this
# script wrote: pickle runs code from the file it reads.
# The integrator's script. About 30 CPU minutes, most of it for the 3000-twist record.
# It writes the two snapshots into <dir> and nothing else. Research only, headless Linux; exact geometry.
# Run from the repository root: python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d31_snapshots.py <dir>
import sys, json, pickle
from pathlib import Path

RUNS = 'research/jumbling/restoration/11-scramble-census/runs/'
SNAPSHOTS = [('s7103-before74.pkl', RUNS + 'round2/ia-k1000-s7103.json', 187,
              '48ef2f688007078600c61370aa42407de4199b707ce29f7b20335487246b7bb8', 94981),
             ('s7302-stop.pkl', RUNS + 'heights/ia-k3000-s7302.json', None,
              '015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f', 100584)]


def load(path, E):
    d = pickle.load(open(path, 'rb'))
    from sim.state import Pose
    C = E.sim.get_context(); M = E.R.menu('I_a')
    st = E.sim.State(C, filtered=False, menu=M)
    st.pose_id = d['pose_id'].copy()
    st._poses = [Pose(k, m, key) for k, m, key in d['poses']]
    st._lookup = {p.key: i for i, p in enumerate(st._poses)}
    assert st.digest() == d['digest'], 'snapshot digest mismatch'
    assert int(st.off_lattice_count()) == d['off']
    return st, {k: d[k] for k in ('record', 'history', 'moves', 'digest', 'off')}


def main(out):
    sys.path[:0] = ['research/jumbling/restoration/12-low-height/scripts', 'research/jumbling/theory',
                    'research/jumbling', 'research/jumbling/fixtures']
    import eb6 as E
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    C = E.sim.get_context(); M = E.R.menu('I_a')
    for name, rec, upto, digest, off in SNAPSHOTS:
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


if __name__ == '__main__':
    main(sys.argv[1])
