"""Math audit: the owner's restoration of the 17-block after the 87-block (prefix 36593 -> 38550).

From prefix-36563.json, applies the owner's twists 36564..36593 (window 36568's witness) by the exact simulator, then
reads the 20-off state 36593: off pieces, holes. U = owner word[36593:38550]. Computes Bad(U) and Bad(U^-1) over all
chambers (chamber ghost, word only), the 17-block's landing chambers at 38550 (from prefix-38550.json), and tests:
Bad(U) inside holes(36593) ((U1) for a word with zero end collateral), 17 <= |Bad(U)| <= 20, landing inside Bad(U^-1).
Usage: python3 -I owner36593.py <theory dir> <v26 dir> <prefix-36563.json> <prefix-38550.json> <owner word json>
       <unlock json> <out json>
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
V26 = Path(sys.argv[2]).resolve()
T36563, T38550, WORD, UNL, OUT = (Path(a) for a in sys.argv[3:8])
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(V26))
import buffer as B  # noqa: E402
import ghostlib as GL  # noqa: E402
from exact import matmul, identity  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()
ID = to_tuple(identity(4))


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


unl = json.loads(UNL.read_text())
wd = json.loads(WORD.read_text())
word = [(int(c), int(i)) for c, i in wd['word']]
out = {'word_length': len(word), 'word_input_digest': wd.get('input_digest'), 'unlock_input_digest': unl['input_digest']}
w68 = unl['windows']['36568']
wit = [(int(c), int(i)) for c, i in w68['witness']['word']]
out['witness_equals_word_36563_36593'] = word[36563:36593] == wit

ctx = GL.Ctx(str(T36563))
st = ctx.st
out['digest_36563'] = st.digest()
out['digest_36563_expected'] = w68['start']['digest']
for c, i in word[36563:36593]:
    ctx.R.apply(c, i)
out['digest_36593'] = st.digest()
out['digest_36593_expected'] = w68['at_close']['digest']
K = st._kpose()[st.pose_id]
off = np.flatnonzero(K < 0)
holes = np.asarray(B.hole_set(st), np.int64)
out['off_36593'] = int(len(off))
out['holes_36593'] = int(len(holes))
log(**{k: v for k, v in out.items()})


def inv_letter(c, i):
    m = ctx.E(c, i)
    js = [j for j in range(len(ctx.M.items)) if to_tuple(matmul(m, ctx.E(c, j))) == ID]
    return (c, js[0])


U = word[36593:38550]
Uinv = [inv_letter(c, i) for c, i in reversed(U)]
allc = np.arange(len(st.pose_id))
bU, _, _ = ctx.bad(U, allc)
bUi, _, _ = ctx.bad(Uinv, allc)
log(badU=len(bU), badUinv=len(bUi))

c38 = GL.Ctx(str(T38550))
out['digest_38550'] = c38.st.digest()
out['digest_38550_expected'] = unl['windows']['38553']['start']['digest']
K38 = c38.st._kpose()[c38.st.pose_id]
off38 = np.flatnonzero(K38 < 0)
reg, kp = GL.BL.CL.ctx.regions, GL.BL.CL.ctx.kplus
ORB, TRN, POF = np.asarray(reg.orbit_of), np.asarray(reg.transport), reg.piece_of
restored = np.setdiff1d(off, off38)
L = POF[ORB[restored], kp.compose_vec(K38[restored], TRN[restored])]
out.update({'off_38550': sorted(off38.tolist()), 'off_38550_subset_off_36593': bool(np.isin(off38, off).all()),
            'restored_36593_38550': int(len(restored)), 'restored_smallest': int(restored.min()) if len(restored) else None,
            'lattice_36593_on_K_at_38550': bool((K38[K >= 0] >= 0).all()),
            'U_length': len(U), 'badU': int(len(bU)), 'badUinv': int(len(bUi)),
            'badU_subset_holes_36593': bool(np.isin(bU, holes).all()),
            'badU_equals_holes_36593': sorted(bU.tolist()) == sorted(holes.tolist()),
            'landing_subset_badUinv': bool(np.isin(L, bUi).all()),
            'landing_meets_badU': int(np.isin(L, bU).sum())})
out['seconds'] = round(time.time() - T0, 1)
OUT.write_text(json.dumps(out, indent=1) + '\n')
log(**{k: v for k, v in out.items() if not isinstance(v, list)})
