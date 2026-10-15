"""Audit 'certificate', extra checks: W is its own mirror inverse; the negative control's three collateral pieces end
at B''s exact E2 pose (a swap); the certificate fingerprint is E2's off geometry; chain-check.json's Bad(U) fields.

Usage: python3 -I extra.py <theory dir> <out json>
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
S = Path('<work>')
B4 = S / 'v26/B4'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(S / 'v26'))
import buffer as B  # noqa: E402
import conjprep as CP  # noqa: E402
from blocks import O  # noqa: E402
from sim.kplus import matrix_json, to_tuple  # noqa: E402

BPRIME = [16245, 16249, 32088]
OCC = [69058, 69059, 121428]
out = Path(sys.argv[2])
R, M = O.start(str(B4 / 'E2-table.json'))
st = R.st
parts = {n: [tuple(int(v) for v in x) for x in json.loads((B4 / 'c3/parts' / f'{n}.json').read_text())['word']]
         for n in ('V', 'P', 'Q', 'Pinv', 'Vinv')}
W = parts['P'] + parts['Q'] + parts['Pinv']
res = {'W_is_own_mirror_inverse': CP.inverse_word(M, W) == W}
cert = json.loads((B4 / 'c3/certificate.json').read_text())
res['off_fingerprint_E2'] = B.off_fingerprint(st)
res['certificate_fingerprint_matches'] = res['off_fingerprint_E2'] == cert['off_geometry_fingerprint']
bpose = {p: to_tuple(st.pose(p)) for p in BPRIME}
res['Bprime_single_pose'] = len(set(bpose.values())) == 1
res['Bprime_pose_E2'] = matrix_json(st.pose(BPRIME[0]))
res['occupants_on_K_at_E2'] = [bool(st._kpose()[st.pose_id[p]] >= 0) for p in OCC]
for c, i in W:
    R.apply(c, i)
K = st._kpose()[st.pose_id]
res['neg_off_after_W'] = np.flatnonzero(K < 0).tolist()
res['neg_occupants_end_at_Bprime_E2_pose'] = [to_tuple(st.pose(p)) == bpose[BPRIME[0]] for p in OCC]
res['neg_Bprime_on_K'] = [bool(K[p] >= 0) for p in BPRIME]
for _ in W:
    R.undo()
cc = json.loads((B4 / 'c3/chain-check.json').read_text())
res['chain_check'] = {'badU': cc.get('badU'), 'badU_equals_E2_holes': cc.get('badU_equals_E2_holes'),
                      'E2_holes': cc.get('E2_holes'), 'word_length': cc.get('word_length')}
out.write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps(res)[:1500])
