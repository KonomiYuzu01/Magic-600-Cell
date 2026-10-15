"""Local sub-history of the W-J I_a walk around one block of its n-record state.

Caps are compared by their solved-state contents (lattice pieces inside each grip's cap). The kept records are those
whose cap is connected, in the cap-overlap graph of the first n records, to a cap of a grip that holds the block
(inside grips from bclass.py). Records outside that component act on a region disjoint from every kept cap, so they
commute with the kept records, and the kept sub-history replays to the n-record state restricted to the kept region
(the replay below certifies every twist admissible on the exact simulator). Writes the sub-history's pose table.

usage: extract.py <research/jumbling> <E-chain scripts> <n> <bclass json> <block first piece> <out table>
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
J, SC, N, BC, FIRST, OUT = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4], int(sys.argv[5]), sys.argv[6]
for p in (J + '/theory', J, J + '/fixtures', SC):
    sys.path.insert(0, p)
import buffer as BU  # noqa: E402
import restore  # noqa: E402
import sim  # noqa: E402
import wj  # noqa: E402

ctx = restore.ctx
fixture, M = wj.load(ctx, 'I_a')
recs = fixture['journal']['records'][:N]
st0 = sim.State(ctx, menu=M)
capset = {}


def cap(c):
    if c not in capset:
        capset[c] = set(st0.classify(int(c), need_inside=True, stop_at_straddle=True).inside.tolist())
    return capset[c]


row = next(r for r in json.loads(Path(BC).read_text())['rows'] if r['first'] == FIRST)
grips = [int(sim.Twist.from_record(ctx, r).grip) for r in recs]
seed = set()
for g in row['inside']:
    seed |= cap(g)
keep, region = set(), set(seed)
changed = True
while changed:
    changed = False
    for k, g in enumerate(grips):
        if k not in keep and cap(g) & region:
            keep.add(k)
            region |= cap(g)
            changed = True
st = sim.State(ctx, menu=M)
for k in sorted(keep):
    assert st.apply(sim.Twist.from_record(ctx, recs[k])).applied, k
BU.export_pose_table(st, OUT)
off = int((st._kpose()[st.pose_id] < 0).sum())
print(json.dumps({'n': N, 'block_first': FIRST, 'block_size': row['size'], 'inside': row['inside'],
                  'kept_records': sorted(keep), 'kept_grips': [grips[k] for k in sorted(keep)],
                  'dropped': len(recs) - len(keep), 'off': off, 'digest': st.digest(), 'table': OUT}))
