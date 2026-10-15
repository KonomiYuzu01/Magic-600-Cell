"""Skeptic audit of MATH-01: is the owner's 87-first-then-17 order available at a successor of X?

One restorer chain, every twist applied by the exact simulator (RuntimeError on an inadmissible twist):
  36563 --u--> X' (must equal X-table's digest) --u^-1--> 36563 --w(36568 witness)--> 36593 --u--> Y*
  --u^-1--> 36593 --U = owner word[36593:38550]--> 38550 (must equal the 38550 table's digest).
Exact poses are compared entry by entry through canonical matrix keys. Holes are chambers no lattice piece occupies.
Usage: python3 -I chain.py <theory dir> <prefix-36563.json> <prefix-38550.json> <owner word json> <unlock json>
       <X-table.json> <X-reachability.json> <out json>
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
T36563, T38550, WORD, UNL, XTAB, XREACH, OUT = (Path(a) for a in sys.argv[2:9])
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import blocks as BL  # noqa: E402
import restore  # noqa: E402
from exact import transpose  # noqa: E402
from sim.kplus import matrix_json, to_tuple  # noqa: E402

CL, O = BL.CL, BL.O
REG, KP = CL.ctx.regions, CL.ctx.kplus
ORB, TRN = np.asarray(REG.orbit_of), np.asarray(REG.transport)
T0 = time.time()
GK = {}
BPRIME = np.array([16245, 16249, 32088])


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def keys(st):
    """Per piece an integer id of its exact pose matrix (global across states)."""
    u = np.unique(st.pose_id)
    lut = np.full(int(u.max()) + 1, -1, np.int64)
    for pid in u.tolist():
        lut[pid] = GK.setdefault(json.dumps(matrix_json(st._pose_matrix(pid))), len(GK))
    return lut[st.pose_id]


def snap(st):
    K = st._kpose()[st.pose_id]
    lat = np.flatnonzero(K >= 0)
    ch = np.asarray(REG.piece_of[ORB[lat], KP.compose_vec(K[lat], TRN[lat])])
    occ = np.zeros(len(K), bool)
    occ[ch] = True
    return {'digest': st.digest(), 'K': K, 'off': np.flatnonzero(K < 0), 'keys': keys(st),
            'distinct_lattice_chambers': bool(len(np.unique(ch)) == len(ch)), 'holes': np.flatnonzero(~occ)}


unl = json.loads(UNL.read_text())
wd = json.loads(WORD.read_text())
word = [(int(c), int(i)) for c, i in wd['word']]
w68 = [(int(c), int(i)) for c, i in unl['windows']['36568']['witness']['word']]
cert = json.loads(XREACH.read_text())
u = [(int(c), int(i)) for c, i in cert['u']]
xdoc = json.loads(XTAB.read_text())
out = {'owner_word_length': len(word), 'owner_word_input_digest': wd['input_digest'],
       'witness_equals_owner_word_36563_36593': word[36563:36593] == w68, 'u_length': len(u)}

R, M = O.start(str(T36563))
st = R.st
INV = {}


def inv(c, i):
    if (c, i) not in INV:
        t = to_tuple(transpose(restore.element(M, c, i)))
        js = [j for j in range(len(M.items)) if restore.element(M, c, j) == t]
        assert len(js) == 1, (c, i, js)
        INV[(c, i)] = (c, js[0])
    return INV[(c, i)]


def run(word_, label):
    for n, (c, i) in enumerate(word_):
        try:
            R.apply(c, i)
        except RuntimeError as e:
            raise SystemExit(f'{label}: twist {n} ({c}, {i}) inadmissible: {e}')
    log(stage=label, twists=len(word_), off=int(st.off_lattice_count()))


u_kplus = [KP.index_of_matrix(restore.element(M, c, i)) >= 0 for c, i in u]
out['u_all_letters_in_Kplus'] = bool(all(u_kplus))
uinv = [inv(c, i) for c, i in reversed(u)]

s0 = snap(st)
out['d36563'] = s0['digest']
log(stage='36563', off=len(s0['off']))
blocks0 = [np.asarray(b, np.int64) for b in BL.Blocks(R, M).blocks]
sizes = sorted(len(b) for b in blocks0)
B87 = next(b for b in blocks0 if len(b) == 87)
B17 = next(b for b in blocks0 if len(b) == 17)
out['blocks_36563'] = sizes
out['B17_smallest'] = int(B17.min())
out['B87_smallest'] = int(B87.min())
out['Bprime_is_3_block'] = sorted(next(b for b in blocks0 if len(b) == 3).tolist()) == BPRIME.tolist()

run(u, 'u from 36563')
sx = snap(st)
out['dX_from_u'] = sx['digest']
out['X_table_digest'] = xdoc['digest']
out['X_cert_digest'] = cert['X_digest']
out['X_matches'] = sx['digest'] == xdoc['digest'] == cert['X_digest']
out['X_off'] = len(sx['off'])
out['X_vs_36563_off_pose_changed'] = {
    'B87': int((sx['keys'][B87] != s0['keys'][B87]).sum()), 'B17': int((sx['keys'][B17] != s0['keys'][B17]).sum()),
    'Bprime': int((sx['keys'][BPRIME] != s0['keys'][BPRIME]).sum())}

run(uinv, 'u^-1 from X')
out['back_to_36563'] = st.digest() == s0['digest']

run(w68, 'window 36568 witness')
s1 = snap(st)
out['d36593'] = s1['digest']
out['d36593_expected'] = unl['windows']['36568']['at_close']['digest']
out['off_36593'] = len(s1['off'])
out['holes_36593'] = len(s1['holes'])
out['lattice_chambers_distinct_36593'] = s1['distinct_lattice_chambers']
out['off_36593_is_B17_and_Bprime'] = sorted(s1['off'].tolist()) == sorted(B17.tolist() + BPRIME.tolist())
chg01 = np.flatnonzero(s1['keys'] != s0['keys'])
out['36563_to_36593_pose_changed_count'] = int(len(chg01))
out['36563_to_36593_pose_changed_is_B87'] = sorted(chg01.tolist()) == sorted(B87.tolist())
out['lattice_36563_on_K_at_36593'] = bool((s1['K'][s0['K'] >= 0] >= 0).all())
out['B87_on_K_at_36593'] = bool((s1['K'][B87] >= 0).all())

run(u, 'u from 36593 (Y*)')
sy = snap(st)
out['dYstar'] = sy['digest']
out['off_Ystar'] = len(sy['off'])
out['holes_Ystar'] = len(sy['holes'])
chgXY = np.flatnonzero(sy['keys'] != sx['keys'])
out['X_to_Ystar_pose_changed_count'] = int(len(chgXY))
out['X_to_Ystar_pose_changed_is_B87'] = sorted(chgXY.tolist()) == sorted(B87.tolist())
out['Ystar_B17_Bprime_exact_X_pose'] = bool((sy['keys'][B17] == sx['keys'][B17]).all()
                                            and (sy['keys'][BPRIME] == sx['keys'][BPRIME]).all())
out['lattice_X_on_K_at_Ystar'] = bool((sy['K'][sx['K'] >= 0] >= 0).all())
out['B87_on_K_at_Ystar'] = bool((sy['K'][B87] >= 0).all())

run(uinv, 'u^-1 from Y*')
out['Ystar_u_inv_reaches_36593'] = st.digest() == s1['digest']

U = word[36593:38550]
out['U_length'] = len(U)
run(U, 'owner word 36593..38550')
s2 = snap(st)
out['d38550'] = s2['digest']
out['d38550_expected'] = unl['windows']['38553']['start']['digest']
out['d38550_table'] = json.loads(T38550.read_text())['digest']
out['off_38550'] = sorted(s2['off'].tolist())
out['off_38550_is_Bprime'] = out['off_38550'] == BPRIME.tolist()
out['lattice_36593_on_K_at_38550'] = bool((s2['K'][s1['K'] >= 0] >= 0).all())
out['lattice_Ystar_on_K_at_38550'] = bool((s2['K'][sy['K'] >= 0] >= 0).all())
out['B17_on_K_at_38550'] = bool((s2['K'][B17] >= 0).all())
out['Bprime_pose_same_36593_38550'] = bool((s2['keys'][BPRIME] == s1['keys'][BPRIME]).all())
out['Bprime_pose_same_X_38550'] = bool((s2['keys'][BPRIME] == sx['keys'][BPRIME]).all())
lat1 = np.flatnonzero(s1['K'] >= 0)
out['lattice_36593_pose_changed_by_U'] = int((s2['keys'][lat1] != s1['keys'][lat1]).sum())
out['holes_36593'] = s1['holes'].tolist()
out['seconds'] = round(time.time() - T0, 1)
OUT.write_text(json.dumps(out, indent=1) + '\n')
log(**{k: v for k, v in out.items() if not isinstance(v, list)})
