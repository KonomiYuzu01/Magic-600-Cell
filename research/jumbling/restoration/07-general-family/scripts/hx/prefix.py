"""Export the pose table of the W-J I_a walk after its first n records.

usage: prefix.py <research/jumbling> <E-chain scripts> <n> <out table>
"""
import sys
from pathlib import Path

sys.dont_write_bytecode = True
J, SC, N, OUT = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
for p in (J + '/theory', J, J + '/fixtures', SC):
    sys.path.insert(0, p)
import buffer as BU  # noqa: E402
import restore  # noqa: E402
import sim  # noqa: E402
import wj  # noqa: E402

ctx = restore.ctx
fixture, M = wj.load(ctx, 'I_a')
st = sim.State(ctx, menu=M)
for rec in fixture['journal']['records'][:N]:
    assert st.apply(sim.Twist.from_record(ctx, rec)).applied
BU.export_pose_table(st, OUT)
print(st.digest())
