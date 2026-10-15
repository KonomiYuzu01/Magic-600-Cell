"""Export exact pose tables of the deep I_a samples on the history of E3: the I_a@40 fixture state (first 40 journal
records of wj-I_a), the seams stop (62 more moves) and the E2-run end (24 more moves). Every twist is checked
admissible and every record is checked to be an I_a menu twist.
usage: export_deep.py <repo root> <out dir>"""
import json, sys
from pathlib import Path
REPO = Path(sys.argv[1]).resolve()
TH = REPO / 'research/jumbling/theory'
sys.path[:0] = [str(TH), str(REPO / 'research/jumbling'), str(REPO / 'research/jumbling/fixtures')]
sys.dont_write_bytecode = True
import sim, wj
import restore as RS
from sim.kplus import matrix_json
ctx = sim.get_context()
fixture, M = wj.load(ctx, 'I_a')
OUT = Path(sys.argv[2])
st = sim.State(ctx, menu=M)
menus = {}
def letter(c, i):
    if c not in menus:
        menus[c] = M.for_cap(c)
    return menus[c][i]
def export(name):
    doc = {'poses': [matrix_json(st._pose_matrix(i)) for i in range(len(st._poses))], 'pose_id': st.pose_id.tolist(),
           'digest': st.digest(), 'journal': []}
    (OUT / f'{name}-table.json').write_text(json.dumps(doc) + '\n')
    print(name, st.digest(), 'off', int(st.off_lattice_count()), flush=True)
for r in fixture['journal']['records'][:40]:
    tw = sim.Twist.from_record(ctx, r)
    assert tw.matrix in [m.matrix for m in M.for_cap(tw.grip)]
    assert st.apply(tw, journal=False).applied
export('fixture40')
stop = json.loads((TH / 'results/seams-I_a-40-stop.json').read_text())['moves']
for c, i in stop:
    assert st.apply(letter(c, i), journal=False).applied
export('stop62')
e2 = json.loads((TH / 'results/joint-attack-I_a-40.json').read_text())['e2_stop_no_history']['moves']
for c, i in e2:
    assert st.apply(letter(c, i), journal=False).applied
export('e2run24')
