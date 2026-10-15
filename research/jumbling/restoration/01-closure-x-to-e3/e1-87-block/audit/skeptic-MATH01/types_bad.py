"""Skeptic audit of MATH-01, part 2: chamber types of the holes and the blocks, and Bad(U) for the owner's U.

Rebuilds 36593 (window 36568 witness at 36563) and Y* = u(36593) by the exact simulator, reads X from X-table.json,
and compares hole sets and K+ chamber types (regions.orbit_of). Then Bad(U) over all chambers for
U = owner word[36593:38550] by the chamber ghost (ghostlib.Ctx.bad; word only), against the holes of 36593.
Usage: python3 -I types_bad.py <theory dir> <v26 dir> <prefix-36563.json> <owner word json> <unlock json>
       <X-table.json> <X-reachability.json> <out json>
"""
import json
import sys
import time
from collections import Counter
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
V26 = Path(sys.argv[2]).resolve()
T36563, WORD, UNL, XTAB, XREACH, OUT = (Path(a) for a in sys.argv[3:9])
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(V26))
import blocks as BL  # noqa: E402
import ghostlib as GL  # noqa: E402

CL, O = BL.CL, BL.O
REG, KP = CL.ctx.regions, CL.ctx.kplus
ORB, TRN = np.asarray(REG.orbit_of), np.asarray(REG.transport)
T0 = time.time()
BPRIME = [16245, 16249, 32088]


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def holes(st):
    K = st._kpose()[st.pose_id]
    lat = np.flatnonzero(K >= 0)
    ch = np.asarray(REG.piece_of[ORB[lat], KP.compose_vec(K[lat], TRN[lat])])
    occ = np.zeros(len(K), bool)
    occ[ch] = True
    return np.flatnonzero(~occ), np.flatnonzero(K < 0)


def types(ids):
    return dict(sorted(Counter(int(t) for t in ORB[np.asarray(ids, np.int64)]).items()))


unl = json.loads(UNL.read_text())
word = [(int(c), int(i)) for c, i in json.loads(WORD.read_text())['word']]
w68 = [(int(c), int(i)) for c, i in unl['windows']['36568']['witness']['word']]
u = [(int(c), int(i)) for c, i in json.loads(XREACH.read_text())['u']]
out = {}

RX, MX = O.start(str(XTAB))
hX, offX = holes(RX.st)
blocks = [np.asarray(b, np.int64) for b in BL.Blocks(RX, MX).blocks]
B87 = next(b for b in blocks if len(b) == 87)
B17 = next(b for b in blocks if len(b) == 17)
out['X_digest'] = RX.st.digest()
out['types_B87'] = types(B87)
out['types_B17'] = types(B17)
out['types_Bprime'] = types(BPRIME)
out['types_holes_X'] = types(hX)
out['types_off_X'] = types(offX)
t87 = set(out['types_B87'])
t20 = set(out['types_B17']) | set(out['types_Bprime'])
out['B87_types_disjoint_from_B17_Bprime'] = not (t87 & t20)
del RX

ctx = GL.Ctx(str(T36563))
R = ctx.R
for c, i in w68:
    R.apply(c, i)
h1, off1 = holes(R.st)
out['d36593'] = R.st.digest()
out['holes_36593'] = h1.tolist()
out['types_holes_36593'] = types(h1)
for c, i in u:
    R.apply(c, i)
hY, offY = holes(R.st)
out['dYstar'] = R.st.digest()
out['holes_Ystar'] = hY.tolist()
out['types_holes_Ystar'] = types(hY)
out['holes_Ystar_subset_holes_X'] = bool(np.isin(hY, hX).all())
out['holes_X_minus_Ystar_types'] = types(np.setdiff1d(hX, hY))
out['holes_X_of_B17_Bprime_types_equal_holes_Ystar'] = sorted(
    h for h in hX.tolist() if int(ORB[h]) in t20) == sorted(hY.tolist())
log(**{k: v for k, v in out.items() if not isinstance(v, list)})

U = word[36593:38550]
allc = np.arange(len(R.st.pose_id))
bU, _, _ = ctx.bad(U, allc)
out['badU'] = sorted(bU.tolist())
out['badU_count'] = int(len(bU))
out['badU_subset_holes_36593'] = bool(np.isin(bU, h1).all())
out['types_badU'] = types(bU)
out['seconds'] = round(time.time() - T0, 1)
OUT.write_text(json.dumps(out, indent=1) + '\n')
log(**{k: v for k, v in out.items() if not isinstance(v, list)})
