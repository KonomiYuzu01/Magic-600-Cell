"""Checks two structural facts (menu I_a, exact): (1) the right factor of every element of every Lambda_c and of every
pose of a pose table lies in 2I (tree distance 0); (2) left factors of Lambda_c fix w_c, and Lambda_c permutes the
grips of its own tree branch (exact pole images).
usage: rightpart.py <research/jumbling> <pose table json> <out json>"""
import json, sys, time
from collections import Counter
from pathlib import Path
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
import restore as RS
from sim.kplus import matrix_from_json
from exact import matmul
import trees as T
t0 = time.time()
ctx = sim.get_context(); d = ctx.data
M = RS.menu('I_a')
tab = json.load(open(sys.argv[2])); OUT = sys.argv[3]
res = Counter()
JI = list(RS.jumble_items(M))
gbr = [T.branch(T.factors(RS.element(M, c, JI[0]))[0]) for c in range(600)]
N = [tuple(d.N[c]) for c in range(600)]
pole = {v: c for c, v in enumerate(N)}
perm_ok = 0; perm_bad = 0
for c in range(600):
    for i in range(60):
        E = RS.element(M, c, i)
        l, r = T.factors(E)
        res['menu_right_dist_%d' % T.dist(r)] += 1
        if c < 40 and i in JI[:6]:
            # images of the poles of branch gbr[c]
            for c2 in range(600):
                if gbr[c2] != gbr[c]:
                    continue
                v = tuple(sum(E[a][b] * N[c2][b] for b in range(4)) for a in range(4))
                if v in pole and gbr[pole[v]] == gbr[c]:
                    perm_ok += 1
                else:
                    perm_bad += 1
for m in tab['poses']:
    l, r = T.factors(matrix_from_json(m))
    res['pose_right_dist_%d' % T.dist(r)] += 1
out = {'counts': dict(res), 'branch_pole_images_ok': perm_ok, 'branch_pole_images_bad': perm_bad,
       'table': sys.argv[2].split('/')[-1], 'sec': round(time.time() - t0, 1)}
Path(OUT).write_text(json.dumps(out, indent=1) + '\n')
print(json.dumps(out))
