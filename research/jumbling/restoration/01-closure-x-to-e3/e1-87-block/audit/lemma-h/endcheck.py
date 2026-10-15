"""Audit lemma-h: compare the chamber ghost of a word with a recorded end state, piece by piece.

For every lattice piece of the start state Y (chamber z), the ghost of z must be good and end in the chamber that
piece occupies in the end table; every off piece of Y followed as its own content must end where the end table puts
it (chamber if on K, exact pose otherwise). Also: the holes of Y, Bad(word) and the good images of the holes against
the holes of the end state. Both tables are loaded through orient.start, which checks their stored digests.

Usage: python3 -I endcheck.py <theory dir> <v26 dir> <start table> <end table> <word json> <start> <end> <out json>
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
import blocks as BL  # noqa: E402
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

CL = BL.CL
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def chambers_of(st):
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    K = st._kpose()[st.pose_id]
    ch = np.full(len(K), -1, np.int64)
    lat = np.flatnonzero(K >= 0)
    ch[lat] = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
    return ch


def main():
    a = sys.argv[1:]
    t_y, t_e, wfile, s0, s1, outp = a[2], a[3], a[4], int(a[5]), int(a[6]), Path(a[7])
    word = [(int(c), int(i)) for c, i in json.loads(Path(wfile).read_text())['word']][s0:s1]
    C = GL.Ctx(t_y)
    stY = C.st
    chY = chambers_of(stY)
    offY = np.flatnonzero(chY < 0)
    holesY = np.setdiff1d(np.arange(GL.NP), chY[chY >= 0])
    RE, _ = BL.O.start(t_e)
    stE = RE.st
    chE = chambers_of(stE)
    holesE = np.setdiff1d(np.arange(GL.NP), chE[chE >= 0])
    log(digest_Y=stY.digest(), digest_E=stE.digest(), offY=len(offY), offE=int((chE < 0).sum()))
    t = time.time()
    grips = sorted({c for c, _ in word})
    union = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in grips]))
    pd, pq = C.contents(offY)
    g = FG.Ghost(C.F, np.concatenate([pd, np.zeros(len(union), np.int64)]), np.concatenate([pq, union]))
    for c, i in word:
        g.apply(c, C.E(c, i), (c, i))
    n = len(offY)
    good = g.alive[n:] & (g.dom[n:] == 0)
    img = np.arange(GL.NP)
    img[union[good]] = g.q[n:][good]
    badmask = np.zeros(GL.NP, bool)
    badmask[union[~good]] = True
    ghost_seconds = round(time.time() - t, 1)
    latY = np.flatnonzero(chY >= 0)
    lattice_bad = int(badmask[chY[latY]].sum())
    pred = img[chY[latY]]
    match = pred == chE[latY]
    # off pieces of Y
    off_rows = []
    for j, p in enumerate(offY.tolist()):
        on = bool(g.alive[j] and g.dom[j] == 0)
        row = {'piece': p, 'ghost_on_K': on, 'ghost_end': int(g.q[j]) if on else None, 'real_end_chamber': int(chE[p])}
        if not on:
            row['real_pose_unchanged'] = to_tuple(stE.pose(p)) == to_tuple(stY.pose(p))
            row['ghost_unchanged'] = bool(g.alive[j] and g.dom[j] == pd[j] and g.q[j] == pq[j])
        off_rows.append(row)
    off_match = sum(1 for r in off_rows if (r['ghost_on_K'] and r['ghost_end'] == r['real_end_chamber'])
                    or (not r['ghost_on_K'] and r['real_end_chamber'] < 0 and r['real_pose_unchanged']
                        and r['ghost_unchanged']))
    bad = np.flatnonzero(badmask)
    good_holes = np.setdiff1d(holesY, bad)
    res = {'start_table': t_y, 'end_table': t_e, 'digest_Y': stY.digest(), 'digest_E': stE.digest(),
           'twists': len(word), 'lattice_pieces_Y': int(len(latY)), 'lattice_pieces_in_bad_chambers': lattice_bad,
           'lattice_pieces_ghost_end_equals_real_end': int(match.sum()),
           'lattice_mismatches': latY[~match][:20].tolist(),
           'off_pieces_Y': int(len(offY)), 'off_pieces_ghost_matches_real': off_match, 'off_rows_sample': off_rows[:5],
           'holes_Y': holesY.tolist() if len(holesY) <= 100 else int(len(holesY)), 'holes_E': holesE.tolist(),
           'bad': bad.tolist() if len(bad) <= 100 else int(len(bad)), 'bad_size': int(len(bad)),
           'bad_subset_holes_Y': bool(np.isin(bad, holesY).all()), 'bad_equals_holes_Y': bool(np.array_equal(bad, holesY)),
           'good_holes_Y': good_holes.tolist(), 'good_holes_images': img[good_holes].tolist(),
           'good_hole_images_are_holes_E': bool(np.isin(img[good_holes], holesE).all()),
           'ghost_seconds': ghost_seconds, 'seconds': round(time.time() - T0, 1)}
    outp.write_text(json.dumps(res, indent=1) + '\n')
    log(**{k: v for k, v in res.items() if not isinstance(v, list) and k not in ('off_rows_sample',)})


if __name__ == '__main__':
    main()
