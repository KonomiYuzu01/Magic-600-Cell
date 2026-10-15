"""K+ types, the type invariant, and the SAFE-ALL orbit split of each type, read from pose tables only.

For each table: the K+ type of a chamber is the K+ orbit of its home piece (chamber s has the type of piece s). The
type invariant says: for every type, the number of holes of that type equals the number of off pieces whose home
chamber has that type. For the first table (the family state) it also builds the SAFE-ALL alphabet, its orbits, and
for every type the orbits inside it, the frozen chambers (singleton orbits) and, against a reference table with no
off piece, which frozen chambers are frozen only because of the off block.

Usage: python -I types3.py <theory dir> <out json> <family table> [<other tables> ...] [--ref <all-on-K table>]
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
import blocks as BL  # noqa: E402
import buffer as B  # noqa: E402

CL, O = BL.CL, BL.O
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def occupancy(st):
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    K = st._kpose()[st.pose_id]
    lat = np.flatnonzero(K >= 0)
    ch = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
    occ = np.full(len(K), -1, np.int64)
    occ[ch] = lat
    return K, occ


def invariant(table, typ):
    R, M = O.start(table)
    st = R.st
    K, occ = occupancy(st)
    off = np.flatnonzero(K < 0)
    holes = np.flatnonzero(occ < 0)
    nt = int(typ.max()) + 1
    h = np.bincount(typ[holes], minlength=nt)
    o = np.bincount(typ[off], minlength=nt)
    return R, M, st, off, holes, {'digest': st.digest(), 'off': int(len(off)), 'holes': int(len(holes)),
                                  'types_with_holes': {int(t): int(h[t]) for t in np.flatnonzero(h)},
                                  'types_of_off_homes': {int(t): int(o[t]) for t in np.flatnonzero(o)},
                                  'invariant_holds': bool(np.array_equal(h, o))}


def main():
    a = sys.argv[1:]
    ref = None
    if '--ref' in a:
        i = a.index('--ref')
        ref = a[i + 1]
        a = a[:i] + a[i + 2:]
    out, tables = a[1], a[2:]
    typ = np.asarray(CL.ctx.regions.orbit_of)
    nt = int(typ.max()) + 1
    tsize = np.bincount(typ, minlength=nt)
    res = {'types': {'count': nt, 'sizes': {int(t): int(tsize[t]) for t in range(nt)}}, 'tables': {}}
    fam = None
    for t in tables + ([ref] if ref else []):
        R, M, st, off, holes, info = invariant(t, typ)
        res['tables'][Path(t).name] = info
        log(table=Path(t).name, **{k: v for k, v in info.items() if k != 'digest'})
        if fam is None:
            fam = (R, M, st, off, holes)
        if t == ref:
            Gr = B.Alphabet(R, M)
            Gr.orbits()
            res['reference'] = {'table': Path(t).name, 'orbits': int(len(Gr.orbit_size)),
                                'singletons': int((Gr.orbit_size == 1).sum())}
            ref_single = Gr.orbit_size[Gr.orbit] == 1
    R, M, st, off, holes = fam
    R, M = O.start(tables[0])  # reload the family state as the current simulator state
    G = B.Alphabet(R, M)
    G.orbits()
    orb, osz = G.orbit, G.orbit_size
    # every SAFE-ALL orbit lies inside one K+ type
    split = {}
    for o in range(len(osz)):
        ts = np.unique(typ[orb == o]) if osz[o] < 50 else None
        if ts is not None and len(ts) != 1:
            raise AssertionError(f'orbit {o} meets several types')
    per_type = {}
    for t in range(nt):
        os_, cnt = np.unique(orb[typ == t], return_counts=True)
        per_type[t] = [(int(o), int(osz[o]), int(c)) for o, c in zip(os_, cnt)]
        for o, c in zip(os_, cnt):
            if osz[o] != c:
                raise AssertionError(f'orbit {o} is not inside type {t}')
    frozen = np.flatnonzero(osz[orb] == 1)
    off_types = sorted({int(x) for x in typ[off]})
    fam_info = {
        'table': Path(tables[0]).name, 'off_pieces': off.tolist(), 'off_home_types': [int(typ[p]) for p in off],
        'holes': holes.tolist(), 'hole_types': [int(typ[h]) for h in holes],
        'hole_orbits': [int(orb[h]) for h in holes], 'hole_orbit_sizes': [int(osz[orb[h]]) for h in holes],
        'safe_grips': len(G.safe), 'letters': len(G.letters), 'orbits': int(len(osz)),
        'singletons': int((osz == 1).sum()), 'frozen_count': int(len(frozen)),
        'types_of_off_homes': {str(t): {'type_size': int(tsize[t]), 'orbits': per_type[t],
                                        'frozen_chambers': [int(c) for c in frozen if typ[c] == t]}
                               for t in off_types},
        'frozen_by_type': {str(t): int((typ[frozen] == t).sum()) for t in range(nt) if (typ[frozen] == t).any()},
        'nontrivial_orbits_by_type': {str(t): [x for x in per_type[t] if x[1] > 1] for t in range(nt)},
    }
    if ref:
        extra = frozen[~ref_single[frozen]]
        fam_info['frozen_only_because_of_off'] = {'count': int(len(extra)), 'chambers': extra.tolist(),
                                                  'types': [int(typ[c]) for c in extra]}
    res['family'] = fam_info
    res['seconds'] = round(time.time() - T0, 1)
    Path(out).write_text(json.dumps(res, indent=1) + '\n')
    log(done=True, off_types=off_types, frozen=len(frozen),
        extra=fam_info.get('frozen_only_because_of_off', {}).get('count'))


if __name__ == '__main__':
    main()
