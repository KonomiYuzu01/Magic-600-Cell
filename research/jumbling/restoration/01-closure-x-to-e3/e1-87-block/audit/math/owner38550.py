"""Math audit: the owner's clean restoration of B' (prefix 38550 -> 41456, window 38553's witness).

Bad(U') over all chambers for U' = owner word[38550:41456] (chamber ghost, word only) against the hole set of 38550.
By (U1) Bad(U') lies in the 3 holes; by the counting lemma |Bad(U')| >= 3 restored pieces; so equality is predicted.
Usage: python3 -I owner38550.py <theory dir> <v26 dir> <prefix-38550.json> <owner word json> <unlock json> <out json>
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
V26 = Path(sys.argv[2]).resolve()
T38550, WORD, UNL, OUT = (Path(a) for a in sys.argv[3:7])
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(V26))
import buffer as B  # noqa: E402
import ghostlib as GL  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


unl = json.loads(UNL.read_text())
word = [(int(c), int(i)) for c, i in json.loads(WORD.read_text())['word']]
w = unl['windows']['38553']
wit = [(int(c), int(i)) for c, i in w['witness']['word']]
ctx = GL.Ctx(str(T38550))
st = ctx.st
holes = np.asarray(B.hole_set(st), np.int64)
U = word[38550:41456]
out = {'digest_38550': st.digest(), 'expected': w['start']['digest'], 'witness_equals_word_slice': U == wit,
       'off': sorted(np.flatnonzero(st._kpose()[st.pose_id] < 0).tolist()), 'holes': holes.tolist(), 'U_length': len(U)}
bU, _, _ = ctx.bad(U, np.arange(len(st.pose_id)))
out.update({'badU': sorted(bU.tolist()), 'badU_equals_holes': sorted(bU.tolist()) == sorted(holes.tolist()),
            'restored_at_close': w['at_close']['restored'], 'off_at_close': w['at_close']['off']})
out['seconds'] = round(time.time() - T0, 1)
OUT.write_text(json.dumps(out, indent=1) + '\n')
log(**out)
