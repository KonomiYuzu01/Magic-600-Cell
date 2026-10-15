"""Lemma H audit, test A: Bad(U) versus the holes at prefix 38550 of the owner's construction.

U = word[38550:] of e2-to-lattice-word.json. Bad(U) is the chamber ghost (ghostlib/fastghost) over the union of U's
caps; the three off pieces are followed as their own contents in the same ghost (contents are independent).
Writes testA.json in this directory. Read-only on every input.

Usage: python3 -I testA.py <theory dir> <work dir v26 dir> <table> <word json> <start> <out json>
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
from exact import transpose  # noqa: E402
from sim.kplus import matrix_json, to_tuple  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def main():
    table, wfile, start, outp = sys.argv[3], sys.argv[4], int(sys.argv[5]), Path(sys.argv[6])
    wdoc = json.loads(Path(wfile).read_text())
    word = [(int(c), int(i)) for c, i in wdoc['word']]
    U = word[start:]
    res = {'table': table, 'word_file': wfile, 'start': start, 'U_length': len(U),
           'word_length': len(word), 'word_output_digest': wdoc.get('output_digest')}
    t = time.time()
    C = GL.Ctx(table)  # O.start checks that the table reproduces its stored digest
    st = C.st
    res['load_seconds'] = round(time.time() - t, 1)
    res['digest'] = st.digest()
    res['digest_in_file'] = json.loads(Path(table).read_text())['digest']
    off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
    res['off'] = off.tolist()
    res['off_count'] = int(len(off))
    res['off_poses'] = sorted({json.dumps(matrix_json(st.pose(int(p)))) for p in off.tolist()})
    res['blocks'] = [list(map(int, b)) for b in C.bl.blocks]
    holes_ctx = np.flatnonzero(~C.occ)
    holes_buf = B.hole_set(st)
    res['holes'] = holes_buf.tolist()
    res['holes_two_ways_equal'] = bool(np.array_equal(holes_ctx, holes_buf))
    log(digest=res['digest'], off=res['off_count'], holes=len(holes_buf))
    # chamber ghost over the union of U's caps, plus the off pieces as their own contents
    t = time.time()
    grips = sorted({c for c, _ in U})
    chambers = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in grips]))
    pd, pq = C.contents(off)
    g = FG.Ghost(C.F, np.concatenate([pd, np.zeros(len(chambers), np.int64)]), np.concatenate([pq, chambers]))
    n = len(off)
    for k, (c, i) in enumerate(U):
        g.apply(c, C.E(c, i), (c, i))
        if (k + 1) % 250 == 0:
            log(twist=k + 1, alive=int(g.alive.sum()), off_dom=int((g.alive & (g.dom != 0)).sum()))
    res['ghost_seconds'] = round(time.time() - t, 1)
    res['grips'] = grips
    res['chambers_in_union_of_caps'] = int(len(chambers))
    lat = slice(n, None)
    died = ~g.alive[lat]
    endoff = g.alive[lat] & (g.dom[lat] != 0)
    bad = np.sort(chambers[died | endoff])
    hs = set(holes_buf.tolist())
    bs = set(bad.tolist())
    res['bad'] = bad.tolist()
    res['bad_size'] = int(len(bad))
    res['bad_died_straddle'] = np.sort(chambers[died]).tolist()
    res['bad_ended_off'] = np.sort(chambers[endoff]).tolist()
    res['bad_subset_holes'] = bs <= hs
    res['bad_equals_holes'] = bs == hs
    res['bad_minus_holes'] = sorted(bs - hs)
    res['holes_minus_bad'] = sorted(hs - bs)
    res['holes_outside_union_of_caps'] = sorted(hs - set(chambers.tolist()))
    res['off_ghost'] = {'alive': g.alive[:n].tolist(), 'end_domain': g.dom[:n].tolist(),
                        'end_chamber_in_frame': g.q[:n].tolist(), 'died_at': g.died_at[:n].tolist()}
    # good chambers: injectivity of the end map; where good contents end
    good = ~(died | endoff)
    ends = g.q[lat][good]
    res['good_end_chambers_distinct'] = bool(len(np.unique(ends)) == len(ends))
    log(bad=res['bad_size'], bad_subset_holes=res['bad_subset_holes'], bad_equals_holes=res['bad_equals_holes'],
        ghost_seconds=res['ghost_seconds'])
    # SAFE-ALL caps that contain each hole (such a cap holds no off piece and no piece straddles its cut)
    t = time.time()
    G = B.Alphabet(C.R, C.M)
    G.orbits()
    safe = np.asarray(G.safe, np.int64)
    res['safe_all_grips'] = int(len(safe))
    res['hole_safe_caps'] = {str(z): safe[G.home[z, safe]].tolist() for z in holes_buf.tolist()}
    res['hole_orbit_sizes'] = {str(z): int(G.orbit_size[G.orbit[z]]) for z in holes_buf.tolist()}
    res['holes_in_some_safe_cap'] = int(sum(1 for z in holes_buf.tolist() if G.home[z, safe].any()))
    res['alphabet_seconds'] = round(time.time() - t, 1)
    log(holes_in_safe_cap=res['holes_in_some_safe_cap'], orbit_sizes=res['hole_orbit_sizes'])
    # the previous twist, inverted, must give the recorded boundary digest at prefix start - 1
    if start > 0:
        c, i = word[start - 1]
        m = C.E(c, i)
        mt = to_tuple(transpose(m))
        j = next(j for j in range(len(C.M.items)) if C.E(c, j) == mt)
        C.R.apply(c, j)
        res['inverse_of_previous_twist'] = [c, i, j]
        res['digest_at_start_minus_1'] = C.R.st.digest()
        bnd = {b['prefix']: b['digest'] for b in wdoc.get('boundaries', [])}
        res['boundary_digest_at_start_minus_1'] = bnd.get(start - 1)
        res['previous_boundary_matches'] = bnd.get(start - 1) == res['digest_at_start_minus_1']
        log(prev_digest=res['digest_at_start_minus_1'], matches=res['previous_boundary_matches'])
    res['seconds'] = round(time.time() - T0, 1)
    outp.write_text(json.dumps(res, indent=1) + '\n')
    log(done=str(outp))


if __name__ == '__main__':
    main()
