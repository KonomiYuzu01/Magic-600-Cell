"""Seed records from single words (words from solved that end with exactly one off piece), menu I_a.
For each word: replay it from solved on the exact simulator (every twist admissible), read the one off piece q, its pose
K_k^-1 gamma K_r and position s = r(q), its position class and registry orbit, the one vacant chamber (the hole) and
the set of grips the word uses. The restoration word of the seed is the inverse word.
usage: seeds.py <research/jumbling> <classes.json> <arrays.npz> <out json> <singles jsonl> [...]"""
import json
import sys
import time

sys.dont_write_bytecode = True
sys.path.insert(0, __import__('os').path.dirname(__file__))
from e3lib import Lib  # noqa: E402

t0 = time.time()
L = Lib(sys.argv[1], sys.argv[2], sys.argv[3])
OUT = sys.argv[4]
TW = {}


def tw(l):
    if l not in TW:
        TW[l] = L.sim.Twist(L.ctx, l[0], L.RS.element(L.M, l[0], l[1]), 'menu', {'menu': L.M.name})
    return TW[l]


rows, seen = [], set()
for path in sys.argv[5:]:
    for line in open(path):
        if not line.strip():
            continue
        r = json.loads(line)
        word = [tuple(x) for x in r['word']]
        key = tuple(word)
        if key in seen:
            continue
        seen.add(key)
        st = L.sim.State(L.ctx, menu=L.M)
        for l in word:
            assert st.apply(tw(l), journal=False).applied
        lat = L.lattice_chambers(st)
        off = [int(p) for p in range((L.NP)) if lat[p] < 0]
        if len(off) != 1:
            print('skip', r.get('name'), 'off', len(off), flush=True)
            continue
        q = off[0]
        c = L.canon(q, L.fm(st.pose(q)))
        assert c is not None, 'not depth one'
        k, rr, s = c
        holes = L.vacancies(st).tolist()
        assert len(holes) == 1
        row = {'name': r.get('name'), 'length': len(word), 'piece': q, 'piece_orbit': int(L.orb[q]), 'k': int(k),
               'r': int(rr), 's': int(s), 'hole': int(holes[0]), 'grips': sorted({l[0] for l in word}),
               'end_digest': st.digest(), 'word': [list(l) for l in word]}
        row.update(L.describe(s))
        rows.append(row)
        print({x: row[x] for x in row if x not in ('word', 'grips', 'end_digest')}, len(row['grips']), 'grips',
              '%.0fs' % (time.time() - t0), flush=True)
json.dump(rows, open(OUT, 'w'))
print('seeds', len(rows), 'classes', len({x['class'] for x in rows}), '%.0fs' % (time.time() - t0))
