"""Audit lemma-h: which chambers an off piece can overlap, decided by the 600 cut hyperplanes (exact).

For each chosen piece p (exact pose from the table), every cut c is classified against p's posed vertices: inside
(all n_c.x >= KAPPA), outside (all <= KAPPA) or straddled; a float screen with margin 1e-9 decides clear vertices and
exact Q(sqrt5) signs decide the rest. A chamber z can meet the interior of p only if z's signature agrees with p's side
on every cut p does not straddle (otherwise that cut separates their interiors). The script lists, per piece, how
many chambers pass this necessary test, which of them are holes and which are occupied. A piece that no chamber passes
lies outside the interior of every chamber, so it meets no hole and no occupied chamber. Lattice control pieces must
pass exactly their own chamber.

Usage: python3 -I separation.py <theory dir> <table> <out json> [--pieces <json list>] [--controls N]
Default pieces: every off piece of the table.
"""
import json
import random
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import sim  # noqa: E402
from exact import ZERO, matvec  # noqa: E402
from sim.kplus import matrix_from_json  # noqa: E402

ctx = sim.get_context()
kp, reg, data = ctx.kplus, ctx.regions, ctx.data
KAPPA = data.KAPPA
NMAT = np.array([[float(x) for x in data.N[c]] for c in range(600)])
EPS = 1e-9
T0 = time.time()


def sides(verts):
    """Per cut: 1 inside, 0 outside, -1 straddle (exact)."""
    vf = np.array([[float(x) for x in v] for v in verts])
    s = vf @ NMAT.T - float(KAPPA)
    out = np.zeros(600, np.int64)
    for c in range(600):
        col = s[:, c]
        pos, neg = col > EPS, col < -EPS
        sg = set()
        for j in np.flatnonzero(~(pos | neg)).tolist():
            sg.add((sum((data.N[c][k] * verts[j][k] for k in range(4)), ZERO) - KAPPA).sign())
        hp, hn = pos.any() or 1 in sg, neg.any() or -1 in sg
        out[c] = -1 if (hp and hn) else (1 if hp else 0)
    return out


def main():
    a = sys.argv[1:]
    t = json.loads(Path(a[1]).read_text())
    outp = Path(a[2])
    opt = {a[k]: a[k + 1] for k in range(3, len(a) - 1, 2)}
    kidx = np.array([kp.index_of_matrix(matrix_from_json(m)) for m in t['poses']], np.int64)
    pid = np.asarray(t['pose_id'], np.int64)
    K = kidx[pid]
    lat = np.flatnonzero(K >= 0)
    ch = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
    occ = np.zeros(len(K), bool)
    occ[ch] = True
    chamber_of = np.full(len(K), -1, np.int64)
    chamber_of[lat] = ch
    pieces = json.loads(opt['--pieces']) if '--pieces' in opt else np.flatnonzero(K < 0).tolist()
    controls = sorted(random.Random(5).sample(lat.tolist(), int(opt.get('--controls', '0'))))
    sig = data.sigbits  # (NP, 75) packed, bit 7 - c % 8 of byte c // 8
    rows = []
    for p in list(pieces) + controls:
        m = matrix_from_json(t['poses'][pid[p]])
        verts = [matvec(m, v) for v in reg.vertices(p)]
        sd = sides(verts)
        want = np.zeros(600, bool)
        want[sd == 1] = True
        mask = np.zeros(600, bool)
        mask[sd >= 0] = True
        wb = np.packbits(np.concatenate([want, np.zeros(0, bool)]))
        mb = np.packbits(mask)
        ok = (((sig ^ wb[None, :]) & mb[None, :]) == 0).all(axis=1)
        cand = np.flatnonzero(ok)
        rows.append({'piece': int(p), 'lattice': bool(K[p] >= 0), 'own_chamber': int(chamber_of[p]),
                     'straddled_cuts': int((sd < 0).sum()), 'inside_cuts': int((sd == 1).sum()),
                     'candidate_chambers': int(len(cand)), 'candidate_holes': cand[~occ[cand]].tolist(),
                     'candidate_occupied': int(occ[cand].sum()), 'candidates_sample': cand[:10].tolist()})
    off_rows = [r for r in rows if not r['lattice']]
    ctl = [r for r in rows if r['lattice']]
    res = {'table_digest': t['digest'], 'off_checked': len(off_rows), 'holes': int((~occ).sum()),
           'off_with_any_candidate_chamber': sum(1 for r in off_rows if r['candidate_chambers']),
           'off_with_candidate_hole': sum(1 for r in off_rows if r['candidate_holes']),
           'holes_that_some_off_piece_may_meet': sorted({z for r in off_rows for z in r['candidate_holes']}),
           'controls': len(ctl),
           'controls_exactly_own_chamber': sum(1 for r in ctl if r['candidate_chambers'] == 1
                                               and r['candidates_sample'] == [r['own_chamber']]),
           'rows': rows, 'seconds': round(time.time() - T0, 1)}
    outp.write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps({k: v for k, v in res.items() if k != 'rows'}))


if __name__ == '__main__':
    main()
