"""k-seed records (menu I_a): words W admissible from solved that end with k >= 1 off pieces. For each: replay W
from solved on the exact simulator (every twist admissible), and record every off piece with its canonical form
(pose K_k^-1 gamma K_r, position s = r(piece), position class), the k holes and the grips W uses. The restoration
word of the k-seed is W^-1; by Lemma R it restores the k pieces together from any state whose off pieces include
pieces at the same regions (up to a common K+ conjugation) and whose corresponding holes are vacant.
Input rows: {"name": ..., "word": [[grip, item], ...]} or {"name": ..., "inverse_of": [[grip, item], ...]}.
usage: kseeds.py <research/jumbling> <classes.json> <arrays.npz> <out json> <rows json>"""
import json
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from e3lib import Lib  # noqa: E402

t0 = time.time()
L = Lib(sys.argv[1], sys.argv[2], sys.argv[3])
TW = {}


def tw(l):
    l = tuple(l)
    if l not in TW:
        TW[l] = L.sim.Twist(L.ctx, l[0], L.RS.element(L.M, l[0], l[1]), 'menu', {'menu': L.M.name})
    return TW[l]


out = []
for r in json.load(open(sys.argv[5])):
    if 'inverse_of' in r:
        word = [list(L.inv_letter(tuple(l))) for l in reversed(r['inverse_of'])]
    else:
        word = [list(l) for l in r['word']]
    st = L.sim.State(L.ctx, menu=L.M)
    for n, l in enumerate(word):
        assert st.apply(tw(l), journal=False).applied, f"{r['name']}: twist {n} not admissible"
    lat = L.lattice_chambers(st)
    off = [int(p) for p in range(L.NP) if lat[p] < 0]
    pcs = []
    for p in off:
        c = L.canon(p, L.fm(st.pose(p)))
        assert c is not None, f"{r['name']}: piece {p} not depth one"
        k, rr, s = c
        row = {'piece': p, 'piece_orbit_registry_index': int(L.orb[p]), 'k': int(k), 'r': int(rr), 's': int(s)}
        row.update(L.describe(s))
        pcs.append(row)
    holes = L.vacancies(st).tolist()
    assert len(holes) == len(off)
    out.append({'name': r['name'], 'length': len(word), 'word': word, 'k_off': len(off), 'off': pcs, 'holes': holes,
                'grips': sorted({int(l[0]) for l in word}), 'end_digest': st.digest()})
    print(r['name'], 'length', len(word), 'off', len(off), 'classes', [x['class'] for x in pcs], 'holes', holes,
          '%.0fs' % (time.time() - t0), flush=True)
json.dump(out, open(sys.argv[4], 'w'))
