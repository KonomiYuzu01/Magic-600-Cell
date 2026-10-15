"""Export the exact pose table of the state reached from solved by a word of I_a menu letters (grip:item, comma
separated); every twist is checked admissible. usage: export_word.py <research/jumbling> <word> <out json>"""
import json, sys
from pathlib import Path
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
import restore as RS
from sim.kplus import matrix_json
ctx = sim.get_context()
M = RS.menu('I_a')
st = sim.State(ctx, menu=M)
word = [tuple(int(v) for v in w.split(':')) for w in sys.argv[2].split(',')]
for c, i in word:
    tw = sim.Twist(ctx, c, RS.element(M, c, i), 'menu', {'menu': M.name})
    assert st.apply(tw, journal=False).applied
doc = {'poses': [matrix_json(st._pose_matrix(i)) for i in range(len(st._poses))], 'pose_id': st.pose_id.tolist(),
       'digest': st.digest(), 'journal': [], 'word': [list(w) for w in word]}
Path(sys.argv[3]).write_text(json.dumps(doc) + '\n')
print('exported', st.digest(), 'off', int(st.off_lattice_count()))
