"""Supplementary link for the 'tables' audit: replay c3/word-full.json on the simulator from the raw E2 table
(every twist must be admissible; R.apply raises otherwise), compare the end state with E3-table.json piece by
piece as exact matrices, and record at every prefix how many of the 3,091 pieces (lattice pieces of E2 whose
exact pose differs at E3, from chk_indep) are off K. Writes chk_replay.json next to this file.
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
import blocks as BL  # noqa: E402
from sim.kplus import matrix_json  # noqa: E402

E2T = SP / 'v26/B4/E2-table.json'
E3T = SP / 'v26/B4/c3/E3-table.json'
WORD = SP / 'v26/B4/c3/word-full.json'


def flatkey(mj):
    return tuple(x for row in mj for e in row for x in e)


def main():
    O = BL.O
    R, M = O.start(str(E2T))
    st = R.st
    print(json.dumps({'loaded': st.digest(), 't': round(time.time() - T0, 1)}), flush=True)
    sets = np.load(HERE / 'chk_indep_sets.npz')
    moved3 = sets['moved3']
    K0 = st._kpose()[st.pose_id]
    lat2 = K0 >= 0
    w = json.loads(WORD.read_text())
    assert st.digest() == w['input_digest']
    recs = []
    for n, (c, i) in enumerate(w['word'], 1):
        R.apply(c, i)
        K = st._kpose()[st.pose_id]
        recs.append({'prefix': n, 'letter': [c, i], 'off': int((K < 0).sum()),
                     'E2_lattice_off': int((lat2 & (K < 0)).sum()), 'moved3_off': int((K[moved3] < 0).sum())})
    end_digest = st.digest()
    # exact comparison with E3-table.json
    e3 = json.loads(E3T.read_text())
    e3keys = [flatkey(m) for m in e3['poses']]
    e3piece = [e3keys[j] for j in e3['pose_id']]
    cache = {}
    diff = 0
    for p, pid in enumerate(st.pose_id.tolist()):
        k = cache.get(pid)
        if k is None:
            k = flatkey(matrix_json(st._pose_matrix(int(pid))))
            cache[pid] = k
        if k != e3piece[p]:
            diff += 1
    res = {'twists': len(w['word']), 'end_digest': end_digest, 'end_digest_is_word_output': end_digest == w['output_digest'],
           'end_digest_is_E3_field': end_digest == e3['digest'], 'pieces_differing_from_E3_table': diff,
           'max_moved3_off_in_word': max(r['moved3_off'] for r in recs),
           'prefixes_with_moved3_off': [r['prefix'] for r in recs if r['moved3_off']],
           'max_E2_lattice_off_in_word': max(r['E2_lattice_off'] for r in recs),
           'prefixes_with_E2_lattice_off': [r['prefix'] for r in recs if r['E2_lattice_off']],
           'prefix_records': recs, 'seconds': round(time.time() - T0, 1)}
    (HERE / 'chk_replay.json').write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps({k: v for k, v in res.items() if k != 'prefix_records'}), flush=True)


if __name__ == '__main__':
    main()
