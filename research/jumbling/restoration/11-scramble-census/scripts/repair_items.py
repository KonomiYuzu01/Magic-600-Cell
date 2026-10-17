# What the three-twist repairs (c, x)(d, y)(c, z) of census records are made of: for every conj step, whether x, y
# and z are retained twists (items 0 to 11 of menu I_a, 0 the identity) or jumble twists (12 to 59), and which item
# the net turn of c, g(c, z) g(c, x), is (the identity when z undoes x). Reads only the records' moves, so it needs
# no replay. Prints one JSON line per record.
# Run from the repository root:  python3 -I -B repair_items.py <record json> ...
import json
import sys
from collections import Counter
from pathlib import Path

REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures')]
import restore  # noqa: E402
from exact import matmul  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

M = restore.menu('I_a')
assert restore.jumble_items(M) == list(range(12, 60)) and to_tuple(restore.element(M, 0, 0)) == restore.I4
_g = {}


def g(c, i):
    if (c, i) not in _g:
        _g[(c, i)] = to_tuple(restore.element(M, c, i))
    return _g[(c, i)]


def item_of(c, mat):
    t = to_tuple(mat)
    return next(i for i in range(60) if g(c, i) == t)


for path in sys.argv[1:]:
    rec = json.loads(Path(path).read_text())
    pos, kinds, n, z_undoes_x, z_identity, net_retained = 0, Counter(), 0, 0, 0, 0
    for step in rec['steps']:
        word = [tuple(m) for m in rec['moves'][pos:pos + step['twists']]]
        pos += step['twists']
        if step['rule'] != 'conj':
            continue
        (c, x), (d, y), (c2, z) = word
        assert c2 == c
        net = item_of(c, matmul(g(c, z), g(c, x)))
        n += 1
        kinds[''.join('R' if i < 12 else 'J' for i in (x, y, z))] += 1
        z_undoes_x += net == 0
        z_identity += z == 0
        net_retained += net < 12
    print(json.dumps({'run': rec['run'], 'repairs': n, 'kinds_xyz': dict(kinds.most_common()),
                      'z_undoes_x': z_undoes_x, 'z_identity': z_identity,
                      'net_turn_of_c_retained': net_retained}))
