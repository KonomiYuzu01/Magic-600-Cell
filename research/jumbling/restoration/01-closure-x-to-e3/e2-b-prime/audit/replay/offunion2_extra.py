"""Extra: of the 34 block pieces among the 3,091 (30 of the 87-block, 4 of the 17-block), how many were off mid-word."""
import json
import sys
from pathlib import Path
sys.dont_write_bytecode = True
AUD = Path('<work>/v26/B4/audit/replay')
sys.path.insert(0, str(AUD))
import numpy as np  # noqa: E402
import replay_audit as RA  # noqa: E402

SP = RA.SP
keys = RA.Keys()
R, M = RA.O.start(str(RA.E2_TABLE))
st = R.st
kp = st.ctx.kplus
kset = {keys.of_key(RA.norm_key(RA.matrix_json(kp.matrix(k)))) for k in range(len(kp.perms))}
TX = RA.table_codes(SP / 'pkg26/Claude-E-closure-X1-20261012/input/X-table.json', keys)[0]
TE1 = RA.table_codes(SP / 'v26/E2/c0/E1-table.json', keys)[0]
isK = np.zeros(len(keys.keys) + 100000, bool)
isK[list(kset)] = True
offX, offE1 = np.flatnonzero(~isK[TX]), np.flatnonzero(~isK[TE1])
blk87 = np.setdiff1d(offE1, RA.BP)
blk17 = np.setdiff1d(offX, offE1)
lat0 = st._kpose()[st.pose_id] >= 0
c0 = keys.codes(st)
ever_off = np.zeros(len(lat0), bool)
for n in ('V', 'P', 'Q', 'Pinv', 'Vinv'):
    for c, i in json.loads((RA.B4 / 'c3/parts' / f'{n}.json').read_text())['word']:
        R.apply(c, i)
        ever_off |= st._kpose()[st.pose_id] < 0
c3 = keys.codes(st)
moved = np.flatnonzero(lat0 & (c3 != c0))
o87 = np.intersect1d(blk87, moved)
o17 = np.intersect1d(blk17, moved)
res = {'overlap87': int(len(o87)), 'overlap87_ever_off': int(ever_off[o87].sum()),
       'overlap17': int(len(o17)), 'overlap17_ever_off': int(ever_off[o17].sum())}
(AUD / 'offunion2-extra.json').write_text(json.dumps(res) + '\n')
print(json.dumps(res))
