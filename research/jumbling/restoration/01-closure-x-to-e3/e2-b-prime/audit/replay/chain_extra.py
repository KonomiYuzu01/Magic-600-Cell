"""Extra: own-parse comparison of X, E1, E2, E3 tables (overlap accounting in the report's certificate chain)."""
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
kp = R.st.ctx.kplus
kset = {keys.of_key(RA.norm_key(RA.matrix_json(kp.matrix(k)))) for k in range(len(kp.perms))}
T = {}
for nm, p in (('X', SP / 'pkg26/Claude-E-closure-X1-20261012/input/X-table.json'), ('E1', SP / 'v26/E2/c0/E1-table.json'),
              ('E2', RA.E2_TABLE), ('E3', RA.E3_TABLE)):
    T[nm] = RA.table_codes(p, keys)[0]
isK = np.zeros(len(keys.keys) + 10, bool)
isK[list(kset)] = True
off = {k: np.flatnonzero(~isK[v]) for k, v in T.items()}
BP = np.array(RA.BP)
blk87 = np.setdiff1d(off['E1'], BP)
blk17 = np.setdiff1d(off['X'], off['E1'])
moved3091 = np.flatnonzero(isK[T['E2']] & (T['E2'] != T['E3']))
res = {'off_counts': {k: int(len(v)) for k, v in off.items()},
       'E1_off_minus_Bprime': int(len(blk87)), 'X_off_minus_E1_off': int(len(blk17)),
       'E1_off_subset_of_X_off': bool(np.isin(off['E1'], off['X']).all()),
       'E2_off_is_Bprime': off['E2'].tolist() == RA.BP,
       'changed_X_E2': int((T['X'] != T['E2']).sum()),
       'changed_X_E2_equals_blocks': bool(np.array_equal(np.flatnonzero(T['X'] != T['E2']), np.union1d(blk17, blk87))),
       'changed_E1_E2': int((T['E1'] != T['E2']).sum()),
       'changed_E1_E2_equals_87block': bool(np.array_equal(np.flatnonzero(T['E1'] != T['E2']), blk87)),
       'moved_E2_E3_lattice': int(len(moved3091)),
       'overlap_87block_with_3091': int(np.isin(blk87, moved3091).sum()),
       'overlap_17block_with_3091': int(np.isin(blk17, moved3091).sum()),
       'changed_E1_E3': int((T['E1'] != T['E3']).sum()), 'changed_X_E3': int((T['X'] != T['E3']).sum())}
(AUD / 'chain-extra.json').write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps(res))
