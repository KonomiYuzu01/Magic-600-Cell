"""Chain check, stage records and Bad set of segment 3 (E2 -> E3), from pose tables and word files only.

- sha256 of segment 2's end table and segment 3's start table; digests of X, E1, E2 and E3 as loaded.
- Exact pose comparison piece by piece (matrix keys) between E2 and E3, E1 and E3, X and E3: the changed sets, B'
  and the 17- and 87-blocks; every piece on K at E3; B''s pose at E3 in K+.
- Stage ends of the replay from E2 (every twist checked admissible by the simulator): digest, off count, B' restored,
  collateral against E2 with its piece list, lattice pieces of E2 at their exact E2 pose.
- Bad(U) of the whole word (word-only chamber ghost over all chambers) against the holes of E2.
- Lattice pieces of E2 whose exact pose changes: count, and whether each one also changes chamber.

Usage: python -I seg3check.py <theory dir> <scripts dir> <X table> <E1 table> <seg-2 end table> <E2 table>
                              <E3 table> <parts dir> <out json>
"""
import hashlib
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
import ghostlib as GL  # noqa: E402
from blocks import CL, O  # noqa: E402
from sim.kplus import matrix_json  # noqa: E402

T0 = time.time()
BPRIME = [16245, 16249, 32088]


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def snapshot(st):
    keys = {}
    pid = np.asarray(st.pose_id)
    out = np.empty(len(pid), object)
    for u in np.unique(pid).tolist():
        keys[u] = json.dumps(matrix_json(st._pose_matrix(int(u))), separators=(',', ':'))
    for i, u in enumerate(pid.tolist()):
        out[i] = keys[u]
    K = st._kpose()[pid]
    return out, K


def chambers(st):
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    K = st._kpose()[st.pose_id]
    ch = np.full(len(K), -1, np.int64)
    lat = np.flatnonzero(K >= 0)
    ch[lat] = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
    return ch


def main():
    a = sys.argv[1:]
    tX, tE1, tS2end, tE2, tE3, parts, outp = a[2], a[3], a[4], a[5], a[6], Path(a[7]), Path(a[8])
    res = {'segment2_end_table_sha256': sha(tS2end), 'segment3_start_table_sha256': sha(tE2),
           'E3_table_sha256': sha(tE3)}
    res['start_tables_identical'] = res['segment2_end_table_sha256'] == res['segment3_start_table_sha256']
    snaps = {}
    for name, t in (('X', tX), ('E1', tE1), ('E3', tE3)):
        R, M = O.start(t)
        snaps[name] = snapshot(R.st)
        res[f'{name}_digest'] = R.st.digest()
        res[f'{name}_off'] = int((snaps[name][1] < 0).sum())
        if name == 'E3':
            ch3 = chambers(R.st)
            res['E3_holes'] = int(len(B.hole_set(R.st)))
            res['Bprime_poses_at_E3'] = {str(p): matrix_json(R.st.pose(p)) for p in BPRIME}
        del R, M
    C = GL.Ctx(tE2)
    R, M, st = C.R, C.M, C.st
    E2keys, E2K = snapshot(st)
    ch2 = chambers(st)
    res['E2_digest'] = st.digest()
    res['E2_off'] = int((E2K < 0).sum())
    res['E2_off_set'] = np.flatnonzero(E2K < 0).tolist()
    holes2 = np.sort(B.hole_set(st))
    res['E2_holes'] = holes2.tolist()
    X3, E13 = snaps['X'][0], snaps['E1'][0]
    E3keys, E3K = snaps['E3']
    res['E3_all_on_K'] = bool((E3K >= 0).all())
    res['Bprime_on_K_at_E3'] = bool((E3K[BPRIME] >= 0).all())
    for name, ref in (('E2', E2keys), ('E1', E13), ('X', X3)):
        changed = np.flatnonzero(ref != E3keys)
        res[f'{name}_to_E3_changed'] = int(len(changed))
        res[f'{name}_to_E3_changed_includes_Bprime'] = bool(np.isin(BPRIME, changed).all())
    ch_e2 = np.flatnonzero(E2keys != E3keys)
    lat2 = E2K >= 0
    moved_lat = ch_e2[lat2[ch_e2]]
    res['E2_lattice_pieces_with_changed_exact_pose'] = int(len(moved_lat))
    res['E2_lattice_pieces_with_changed_exact_pose_ids'] = moved_lat.tolist()
    res['of_them_changed_chamber'] = int((ch2[moved_lat] != ch3[moved_lat]).sum())
    res['E2_lattice_pieces_same_exact_pose'] = int(lat2.sum() - len(moved_lat))
    res['E2_lattice_pieces'] = int(lat2.sum())
    # stage ends
    word = []
    stages = []
    for name in ('V', 'P', 'Q', 'Pinv', 'Vinv'):
        d = json.loads((parts / f'{name}.json').read_text())
        w = [tuple(x) for x in d['word']]
        if st.digest() != d['input_digest']:
            raise AssertionError(f'{name}: input digest differs')
        for c, i in w:
            R.apply(c, i)
        word += w
        keys, K = snapshot(st)
        coll = np.flatnonzero(lat2 & (K < 0))
        stages.append({'stage_end': name, 'twists': len(w), 'end_digest': st.digest(),
                       'output_digest_matches_file': st.digest() == d['output_digest'],
                       'off_end': int((K < 0).sum()), 'Bprime_on_K': int((K[BPRIME] >= 0).sum()),
                       'collateral_vs_E2': int(len(coll)), 'collateral_ids': coll.tolist(),
                       'E2_lattice_same_exact_pose': int((lat2 & (keys == E2keys)).sum())})
    res['stages'] = stages
    res['end_digest_equals_E3'] = st.digest() == res['E3_digest']
    for _ in word:
        R.undo()
    if st.digest() != res['E2_digest']:
        raise AssertionError('undo did not return to E2')
    bad, g, chs = C.bad(word, np.arange(len(C.occ)))
    bad = np.sort(bad)
    res['word_length'] = len(word)
    res['badU'] = bad.tolist()
    res['badU_equals_E2_holes'] = bool(np.array_equal(bad, holes2))
    res['seconds'] = round(time.time() - T0, 1)
    outp.write_text(json.dumps(res) + '\n')
    print(json.dumps({k: v for k, v in res.items() if not isinstance(v, (list, dict))}))


if __name__ == '__main__':
    main()
