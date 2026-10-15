"""Extra: compare E2.V3.P4 (W3 after P4) with E2.P4.V3 (type S after Q0), and the states after F, piece by piece."""
import json
import sys
from pathlib import Path
sys.dont_write_bytecode = True
AUD = Path('<work>/v26/B4/audit/replay')
sys.path.insert(0, str(AUD))
import numpy as np  # noqa: E402
import replay_audit as RA  # noqa: E402

B4 = RA.B4
w = {n: [tuple(x) for x in json.loads((B4 / 'c3/parts' / f'{n}.json').read_text())['word']] for n in ('V', 'P', 'Q')}
keys = RA.Keys()
out = {}
snap = {}
for label, order in (('VP', ['V', 'P', 'Q']), ('PV', ['P', 'V', 'Q'])):
    R, M = RA.O.start(str(RA.E2_TABLE))
    c0 = keys.codes(R.st)
    for n in order:
        for c, i in w[n]:
            R.apply(c, i)
        snap[(label, n)] = (keys.codes(R.st), R.st.digest())
    snap[(label, 'E2')] = (c0, None)
c0 = snap[('VP', 'E2')][0]
a, da = snap[('VP', 'P')]
b, db = snap[('PV', 'V')]
diff = np.flatnonzero(a != b)
out['after_V3P4_vs_P4V3'] = {'digests': [da[:16], db[:16]], 'pieces_differing': int(len(diff)),
                             'exact_E2_counts': [int((a == c0).sum()), int((b == c0).sum())]}
a, da = snap[('VP', 'Q')]
b, db = snap[('PV', 'Q')]
diff2 = np.flatnonzero(a != b)
out['after_F'] = {'digests': [da[:16], db[:16]], 'pieces_differing': int(len(diff2))}
# pieces moved by V3 alone (from E2) and by P4 alone (from E2)
R, M = RA.O.start(str(RA.E2_TABLE))
for c, i in w['V']:
    R.apply(c, i)
cv = keys.codes(R.st)
R, M = RA.O.start(str(RA.E2_TABLE))
for c, i in w['P']:
    R.apply(c, i)
cp = keys.codes(R.st)
mv, mp = np.flatnonzero(cv != c0), np.flatnonzero(cp != c0)
out['moved_by_V3_from_E2'] = int(len(mv))
out['moved_by_P4_from_E2'] = int(len(mp))
out['overlap'] = int(len(np.intersect1d(mv, mp)))
out['differing_pieces_in_V3_support'] = int(np.isin(diff, mv).sum())
out['differing_pieces_in_P4_support'] = int(np.isin(diff, mp).sum())
(AUD / 'order-extra.json').write_text(json.dumps(out, indent=1) + '\n')
print(json.dumps(out))
