# Independent check of template steps on a census residue. Rebuilds the stop state from the census record (scramble
# seed and moves) on the exact simulator, then applies the step words in order and checks for each: the input digest,
# every letter admissible and certain, the step's pieces all on the lattice at the end, no piece that was on the
# lattice taken off, every other off piece at its exact pose, and the output digest. Does not import the code that
# found the words. Run from the repository root:
#   python3 -I -B check_steps.py <census record> <step word json>...
import sys, json
from pathlib import Path
import numpy as np
REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures')]
import restore, sim
from sim.kplus import matrix_json

M = restore.menu('I_a')
rec = json.loads(Path(sys.argv[1]).read_text())
st, hist = restore.scramble(M, rec['k'], rec['seed'])
assert [list(h) for h in hist] == [list(h) for h in rec['history']], 'scramble differs from the record'


def apply(c, i):
    o = st.apply(sim.Twist(restore.ctx, int(c), restore.element(M, c, i), 'menu', {'menu': M.name}))
    return o.applied and not o.uncertain


assert all(apply(c, i) for c, i in rec['moves']), 'a census move is not admissible'
assert int(st.off_lattice_count()) == rec['off_now']


def poses():
    """Exact pose of every piece, as text, one entry per pose id."""
    key = {pid: json.dumps(matrix_json(st._pose_matrix(pid))) for pid in np.unique(st.pose_id).tolist()}
    return [key[pid] for pid in st.pose_id.tolist()]


bad = 0
for f in sys.argv[2:]:
    w = json.loads(Path(f).read_text())
    fails = []
    if st.digest() != w['input_digest']:
        fails.append('input digest')
    on0, pose0 = st.lattice_flags().copy(), poses()
    for c, i in w['word']:
        if not apply(c, i):
            fails.append(f'letter {(c, i)} not admissible')
    on1, pose1 = st.lattice_flags(), poses()
    T = set(int(p) for p in w['pieces'])
    if not all(on1[p] for p in T):
        fails.append('a piece of the step is still off')
    taken_off = int((on0 & ~on1).sum())
    if taken_off:
        fails.append(f'{taken_off} lattice pieces taken off')
    others = [p for p in np.flatnonzero(~on0).tolist() if p not in T]
    moved = sum(1 for p in others if pose0[p] != pose1[p])
    if moved:
        fails.append(f'{moved} other off pieces moved')
    if st.digest() != w['output_digest']:
        fails.append('output digest')
    out = {'step': Path(f).name, 'letters': len(w['word']), 'pieces': len(T), 'off_after': int(st.off_lattice_count()),
           'other_off_pieces_kept': len(others) - moved, 'digest': st.digest(), 'pass': not fails, 'fails': fails}
    bad += bool(fails)
    print(json.dumps(out), flush=True)
sys.exit(1 if bad else 0)
