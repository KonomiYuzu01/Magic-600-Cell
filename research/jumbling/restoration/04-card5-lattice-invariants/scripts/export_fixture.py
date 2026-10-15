"""Export exact pose tables of I_a W-J fixture prefixes (records 1..T, every twist checked admissible and in the
menu), optionally followed by extra I_a letters grip:item. usage: export_fixture.py <research/jumbling> <T list>
<out dir> [extra word]"""
import json, sys
from pathlib import Path
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim, wj
import restore as RS
from sim.kplus import matrix_json
ctx = sim.get_context()
fixture, M = wj.load(ctx, 'I_a')
recs = fixture['journal']['records']
cps = sorted(int(x) for x in sys.argv[2].split(','))
OUT = Path(sys.argv[3])
extra = [tuple(int(v) for v in w.split(':')) for w in sys.argv[4].split(',')] if len(sys.argv) > 4 else []
st = sim.State(ctx, menu=M)
def export(name):
    doc = {'poses': [matrix_json(st._pose_matrix(i)) for i in range(len(st._poses))], 'pose_id': st.pose_id.tolist(),
           'digest': st.digest(), 'journal': []}
    (OUT / f't-{name}.json').write_text(json.dumps(doc) + '\n')
    print(name, st.digest(), 'off', int(st.off_lattice_count()), flush=True)
for T, r in enumerate(recs[:max(cps)]):
    tw = sim.Twist.from_record(ctx, r)
    assert tw.matrix in [m.matrix for m in M.for_cap(tw.grip)]
    assert st.apply(tw, journal=False).applied
    if T + 1 in cps:
        export(f'T{T + 1}')
if extra:
    for c, i in extra:
        tw = sim.Twist(ctx, c, RS.element(M, c, i), 'menu', {'menu': M.name})
        assert st.apply(tw, journal=False).applied
    export(f'T{max(cps)}x' + '_'.join(f'{c}-{i}' for c, i in extra))
