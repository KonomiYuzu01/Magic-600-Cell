"""Simulator cross-check for the 'tables' audit: K+ membership by sim.kplus.index_of_matrix for every pose of
X, E1, E2 and E3; the simulator's 7,200 K+ matrices against the closure computed in chk_indep.py; chambers by
the simulator's region transport (the formula of seg3check.chambers) against the signature chambers of
chk_indep.py. Reads the raw tables directly. Writes chk_sim.json next to this file.
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

T0 = time.time()
SP = Path('<work>')
HERE = SP / 'v26/B4/audit/tables'
TH = SP / 'wt-e3/research/jumbling/theory'
sys.dont_write_bytecode = True
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import sim  # noqa: E402
from sim.kplus import matrix_from_json, matrix_json  # noqa: E402

NP = 177120
PATHS = {
    'X': SP / 'pkg26/Claude-E-closure-X1-20261012/input/X-table.json',
    'E1': SP / 'v26/E2/c0/E1-table.json',
    'E2': SP / 'v26/B4/E2-table.json',
    'E3': SP / 'v26/B4/c3/E3-table.json',
}


def log(**kw):
    kw['t'] = round(time.time() - T0, 1)
    print(json.dumps(kw), flush=True)


def flat(mj):
    return tuple(x for row in mj for e in row for x in e)


def main():
    ctx = sim.get_context()
    kp, reg = ctx.kplus, ctx.regions
    log(step='context')
    res = {}
    closure = {tuple(k) for k in json.loads((HERE / 'closure_keys.json').read_text())}
    simK = {flat(matrix_json(kp.matrix(i))) for i in range(len(kp.perms))}
    res['sim_Kplus_order'] = len(kp.perms)
    res['sim_Kplus_distinct_matrices'] = len(simK)
    res['sim_Kplus_equals_independent_closure'] = simK == closure
    log(step='kplus', equal=res['sim_Kplus_equals_independent_closure'], order=len(simK))
    sets = np.load(HERE / 'chk_indep_sets.npz')
    mine_off = {'X': sets['offX'], 'E1': sets['offE1'], 'E2': sets['offE2'], 'E3': np.array([], np.int64)}
    mine_ch = {'E2': sets['chE2'], 'E3': sets['chE3']}
    Ks = {}
    for name, path in PATHS.items():
        doc = json.loads(Path(path).read_text())
        kidx = np.array([kp.index_of_matrix(matrix_from_json(m)) for m in doc['poses']], np.int64)
        K = kidx[np.asarray(doc['pose_id'], np.int64)]
        Ks[name] = K
        off = np.flatnonzero(K < 0)
        lat = np.flatnonzero(K >= 0)
        ch = np.full(NP, -1, np.int64)
        ch[lat] = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
        occ = np.unique(ch[lat])
        holes = np.setdiff1d(np.arange(NP), occ)
        r = {'off': int(len(off)), 'off_equals_independent': bool(np.array_equal(off, mine_off[name])),
             'collisions': int(len(lat) - len(occ)), 'holes': holes.tolist() if len(holes) <= 10 else int(len(holes))}
        if name in mine_ch:
            r['chambers_equal_independent'] = bool(np.array_equal(ch, mine_ch[name]))
        res[name] = r
        log(step='table', name=name, **r)
    moved3 = sets['moved3']
    res['moved3_sim_K_E3_nonneg'] = bool((Ks['E3'][moved3] >= 0).all())
    res['seconds'] = round(time.time() - T0, 1)
    (HERE / 'chk_sim.json').write_text(json.dumps(res, indent=1) + '\n')
    log(step='done', seconds=res['seconds'])


if __name__ == '__main__':
    main()
