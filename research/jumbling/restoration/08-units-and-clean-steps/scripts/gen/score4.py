"""Score candidate words of one restoration unit at a state, with the conflicting blocks named (menu I_a).

For every word W (rows with 'W' or 'Q') the exact frame ghost of T, of the unit's own free set, of every other off
piece and of every lattice chamber of W's caps gives:
- T_on, T_dead: pieces of T that end on K, pieces of T that W cannot carry (a twist would cut them);
- free_dead: pieces of the unit's free set that W cannot carry;
- dead_blocks: other blocks with a piece that W cannot carry (W is then not executable at this state);
- moved_blocks: other blocks with a piece that W carries but does not return to its exact content. With these blocks
  added to the free set, the step is a partial step of Lemma U: the off count still falls by at least |T|;
- bad, occ, deficit: Bad(W) (lattice chambers of W's caps whose lattice ghost W cannot carry or brings off K), its
  occupied chambers, and the per-type deficit sum_t max(0, occupied Bad of type t - holes of type t outside Bad)
  (Lemma D: a lattice-preserving vacating word exists only if the deficit is 0). occ = 0 means W needs no V;
- n1, n2: the vacancy conditions N1 and N2 of buffer.condition for the SAFE-ALL alphabet of this state (the letters
  of V): n1 counts the occupied Bad chambers that no SAFE-ALL letter moves, n2 = sum over SAFE-ALL orbits o of
  max(0, |Bad_o| - holes_o). A vacating word of SAFE-ALL letters exists only if n1 = n2 = 0 ('vac_ok'); the SAFE-ALL
  orbits refine the K+ types, so vac_ok implies deficit 0.
Rows with T_on < |T|, T_dead > 0 or free_dead > 0 are written with 'usable': false.

usage: score4.py <theory dir> <E-chain scripts dir> <pose table> <blocks json [[first, [pieces]]...]>
       <target json {pieces}> <words jsonl> <out jsonl> [--free <json {pieces}>]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import buffer as B  # noqa: E402
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402
import rim  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


class Lite(GL.Ctx):
    def __init__(self, table):  # noqa: D401 - ghostlib.Ctx without the block computation
        self.R, self.M = GL.O.start(table)
        self.st = self.R.st
        self.D = rim.Domains(self.st)
        self.F = FG.Frames()
        self.dmap = {j: self.F.domain_of(rep) for j, rep in enumerate(self.D.reps)}
        lat = np.flatnonzero(self.D.dom == 0)
        self.occ = np.zeros(GL.NP, bool)
        self.occ[self.D.cham[lat]] = True
        self.off = np.flatnonzero(self.st._kpose()[self.st.pose_id] < 0)
        self._el = {}


table, bfile, tfile, wfile, out = sys.argv[3:8]
a = sys.argv[8:]
C = Lite(table)
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
FREE = np.asarray(sorted(json.loads(Path(a[a.index('--free') + 1]).read_text())['pieces']), np.int64) \
    if '--free' in a else np.zeros(0, np.int64)
OTH = np.setdiff1d(np.setdiff1d(C.off, T), FREE)
blk = np.full(GL.NP, -1, np.int64)
for first, ps in json.loads(Path(bfile).read_text()):
    blk[np.asarray(ps, np.int64)] = int(first)
oblk = blk[OTH]
d0, q0 = C.contents(OTH)
holes = ~C.occ
ORB = np.asarray(GL.data.orbit_id, np.int64) + 1  # 36 K+ chamber orbits; data labels them -1..34
NORB = int(ORB.max()) + 1
nT, nF = len(T), len(FREE)
G = B.Alphabet(C.R, C.M)
G.orbits()
assert np.array_equal(np.sort(G.holes), np.flatnonzero(holes)), 'hole sets differ'
GO = np.asarray(G.orbit, np.int64)
NO = len(G.orbit_size)
SINGLE = G.orbit_size[GO] == 1
HO = np.bincount(GO[np.flatnonzero(holes)], minlength=NO)
seen = set()
n = 0
with open(out, 'w') as fh:
    for line in Path(wfile).read_text().splitlines():
        if not line.strip():
            continue
        r = json.loads(line)
        word = r.get('W') or r.get('Q')
        key = json.dumps(word)
        if key in seen:
            continue
        seen.add(key)
        W = [(int(c), int(i)) for c, i in word]
        g, m, U = C.ghost(W, np.concatenate([T, FREE, OTH]))
        k = nT + nF
        ta, fa, pa = g.alive[:nT], g.alive[nT:k], g.alive[k:m]
        mv = pa & ((g.dom[k:m] != d0) | (g.q[k:m] != q0))
        lat = slice(m, None)
        bad = U[~g.alive[lat] | (g.dom[lat] != 0)]
        occb = bad[C.occ[bad]]
        freeh = holes.copy()
        freeh[bad] = False
        need = np.bincount(ORB[occb], minlength=NORB)
        have = np.bincount(ORB[np.flatnonzero(freeh)], minlength=NORB)
        row = {'W': [list(x) for x in W], 'T_on': int((ta & (g.dom[:nT] == 0)).sum()), 'T_dead': int((~ta).sum()),
               'free_dead': int((~fa).sum()), 'dead_blocks': sorted(set(oblk[~pa].tolist())),
               'moved_blocks': sorted(set(oblk[mv].tolist())), 'moved_pieces': int(mv.sum()),
               'bad': int(len(bad)), 'occ': int(len(occb)), 'deficit': int(np.maximum(need - have, 0).sum()),
               'n1': int((SINGLE[bad] & C.occ[bad]).sum()),
               'n2': int(np.maximum(np.bincount(GO[bad], minlength=NO) - HO, 0).sum()), 'src': r.get('src')}
        row['vac_ok'] = bool(row['n1'] == 0 and row['n2'] == 0)
        row['usable'] = bool(row['T_on'] == nT and row['T_dead'] == 0 and row['free_dead'] == 0
                             and not row['dead_blocks'])
        fh.write(json.dumps(row) + '\n')
        n += 1
log(event='end', words=n, T=nT, free=nF, other_off=int(len(OTH)))
