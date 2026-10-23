# Round 22, line (c): prepare the closure of T1' (the solver's computation C2, given arguments by the integrator). For
# a landing word W1 at Z it exports the pose table of Z1 = W1(Z), the unit file of Q (the lattice pieces W1 took off)
# and the word W1^-1 as a driver W file {"P": [], "Q": word}, for tclose2.py. Exact; headless; research only.
# Usage (from the repository root): python3 -I -B prep_t1prime.py <Z table> <W1 json> <tag> <out dir>
import sys, json
from pathlib import Path
import numpy as np
REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'), str(REPO / 'research/jumbling/fixtures')]
import restore, sim, rim, buffer as B, orient as O
from exact import transpose
from sim.kplus import to_tuple
table, wfile, tag, odir = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
W = json.loads(Path(wfile).read_text())
R, M = O.start(table); st = R.st
assert st.digest() == W['input_digest']
D0 = rim.Domains(st); off0 = np.flatnonzero(np.asarray(D0.dom) != 0)
for c, i in W['word']:
    o = st.apply(sim.Twist(restore.ctx, int(c), restore.element(M, c, i), 'menu', {'menu': M.name}))
    assert o.applied and not o.uncertain
assert st.digest() == W['output_digest']
D1 = rim.Domains(st); off1 = np.flatnonzero(np.asarray(D1.dom) != 0)
newly = np.setdiff1d(off1, off0)
doc = B.export_pose_table(st, f'{odir}/{tag}-z1-table.json')
Path(f'{odir}/{tag}-Q.json').write_text(json.dumps({'state': 'z1', 'digest': st.digest(), 'pieces': [int(p) for p in newly]}))
# inverse word: reversed letters with the inverse item at the same grip (exact matrix comparison; rotations: inverse = transpose)
inv = []
for c, i in reversed(W['word']):
    g = restore.element(M, c, i); gi = to_tuple(transpose(g))
    js = [j for j in range(len(M.items)) if to_tuple(restore.element(M, c, j)) == gi]
    assert len(js) == 1, (c, i, js)
    inv.append([int(c), int(js[0])])
Path(f'{odir}/{tag}-winv.json').write_text(json.dumps({'P': [], 'Q': inv}))
print(json.dumps({'z1_digest': st.digest(), 'table_digest': doc['digest'], 'Q_pieces': int(len(newly)), 'inverse_word_len': len(inv), 'off_z1': int(len(off1))}))
