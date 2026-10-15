"""Extra: are the 10 parked pieces at the same exact poses at Y_4 = E2.P4 and at E2.V3.P4 (the state F acts on in W3)?"""
import json
import sys
from pathlib import Path
sys.dont_write_bytecode = True
AUD = Path('<work>/v26/B4/audit/replay')
sys.path.insert(0, str(AUD))
import numpy as np  # noqa: E402
import replay_audit as RA  # noqa: E402

w = {n: [tuple(x) for x in json.loads((RA.B4 / 'c3/parts' / f'{n}.json').read_text())['word']] for n in ('V', 'P')}
keys = RA.Keys()
st_codes = {}
for label, order in (('Y4', ['P']), ('VP', ['V', 'P'])):
    R, M = RA.O.start(str(RA.E2_TABLE))
    for n in order:
        for c, i in w[n]:
            R.apply(c, i)
    K = R.st._kpose()[R.st.pose_id]
    st_codes[label] = (keys.codes(R.st), np.flatnonzero(K < 0), R.st.digest())
a, offa, da = st_codes['Y4']
b, offb, db = st_codes['VP']
res = {'digests': [da[:16], db[:16]], 'off_Y4': offa.tolist(), 'off_VP': offb.tolist(),
       'same_off_set': bool(np.array_equal(offa, offb)),
       'off_pieces_same_exact_pose': bool(np.array_equal(a[offa], b[offa])) if np.array_equal(offa, offb) else None}
(AUD / 'y4-extra.json').write_text(json.dumps(res) + '\n')
print(json.dumps(res))
