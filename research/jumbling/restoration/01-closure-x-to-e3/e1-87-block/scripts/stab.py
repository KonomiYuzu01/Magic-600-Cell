"""After a type-S P at the current state: can a SAFE-ALL word fixing the bad holes pointwise bring the good holes to
B''s chambers? Orbits of H0, the group of SAFE-ALL letters whose caps avoid every bad hole, and of H1, H0 plus the
commutators [x, y] of SAFE-ALL letters (x's cap holding a chamber of L or GH) whose support avoids the bad holes.
Usage: python -I stab.py <theory dir> <pose table> <P json word> <out json> [--h0-only]"""
import json, sys, time
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent)); sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import buffer as B
import conjprep as CP
import ghostlib as GL
T0 = time.time()
C = GL.Ctx(sys.argv[2])
R, M, st = C.R, C.M, C.st
off0 = C.off.copy()
P = [tuple(x) for x in json.loads(sys.argv[3])]
badPinv, _, _ = C.bad(CP.inverse_word(M, P))
for c, i in P:
    R.apply(c, i)
G = B.Alphabet(R, M); G.orbits()
holes = np.asarray(G.holes)
inbad = np.zeros(G.NP, bool); inbad[badPinv] = True
BH = holes[inbad[holes]]; GH = holes[~inbad[holes]]
from blocks import CL
reg, kp = CL.ctx.regions, CL.ctx.kplus
Kp = st._kpose()[st.pose_id]
L = reg.piece_of[reg.orbit_of[off0], kp.compose_vec(Kp[off0], reg.transport[off0])]
isBH = np.zeros(G.NP, bool); isBH[BH] = True


def components(edges_iter):
    parent = np.arange(G.NP)
    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x
    for xs, ys in edges_iter:
        for a, b in zip(xs.tolist(), ys.tolist()):
            ra, rb = find(a), find(b)
            if ra != rb:
                parent[max(ra, rb)] = min(ra, rb)
    return np.array([find(x) for x in range(G.NP)])


h0_grips = [c for c in G.safe if not isBH[G.capx[c]].any()]
def h0_edges():
    for n, (c, _, _, ys) in enumerate(G.letters):
        if c in h0_grips:
            yield G.capx[c], ys
lab0 = components(h0_edges())
res = {'after_P_digest': st.digest(), 'holes': int(len(holes)), 'BH': int(len(BH)), 'GH': GH.tolist(), 'L': L.tolist(),
       'badPinv': int(len(badPinv)), 'safe_grips': len(G.safe), 'h0_grips': len(h0_grips),
       'L_in_h0_caps': [bool(any(G.home[x, c] for c in h0_grips)) for x in L.tolist()],
       'GH_in_h0_caps': [bool(any(G.home[x, c] for c in h0_grips)) for x in GH.tolist()],
       'L_h0_comp': [int(lab0[x]) for x in L.tolist()], 'GH_h0_comp': [int(lab0[x]) for x in GH.tolist()],
       'G_orbit_L': [int(G.orbit[x]) for x in L.tolist()], 'G_orbit_GH': [int(G.orbit[x]) for x in GH.tolist()],
       'h0_sec': round(time.time() - T0, 1)}
print(json.dumps(res), flush=True)
Path(sys.argv[4]).write_text(json.dumps(res, indent=1) + '\n')
if '--h0-only' in sys.argv:
    sys.exit(0)
# H1: commutators with support avoiding BH, x's cap holding a chamber of L or GH
near = set(L.tolist()) | set(GH.tolist())
xl = [n for n, (c, _, _, _) in enumerate(G.letters) if any(G.home[z, c] for z in near)]
comm = []
ident = np.arange(G.NP)
for x in xl:
    cx = G.letters[x][0]
    for y in range(len(G.letters)):
        cy = G.letters[y][0]
        if cy == cx or not (G.home[:, cx] & G.home[:, cy]).any():
            continue
        w = [x, y, int(G.inv[x]), int(G.inv[y])]
        p = G.perm(w)
        mv = np.flatnonzero(p != ident)
        if len(mv) and not isBH[mv].any():
            comm.append((x, y, mv, p[mv]))
res['commutators_avoiding_BH'] = len(comm)
def h1_edges():
    yield from h0_edges()
    for x, y, mv, pm in comm:
        yield mv, pm
lab1 = components(h1_edges())
res['L_h1_comp'] = [int(lab1[x]) for x in L.tolist()]
res['GH_h1_comp'] = [int(lab1[x]) for x in GH.tolist()]
res['comm_touching_L'] = sum(1 for _, _, mv, _ in comm if np.isin(L, mv).any())
res['comm_touching_GH'] = sum(1 for _, _, mv, _ in comm if np.isin(GH, mv).any())
res['sec'] = round(time.time() - T0, 1)
Path(sys.argv[4]).write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps(res))
