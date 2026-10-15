"""Certificate TREEFACTS (menu I_a, exact Q(sqrt5) arithmetic): the tree facts used by Lemma R, Lemma P, AL, Lemma L,
Lemma FN and Proposition HB.
  T1  every grip normal n_c moves v0 one step (d(n_c v0, v0) = 1); the 600 grips fall into 5 branches of 120;
  T2  every unit icosian fixes v0;
  T3  for every grip c and every menu element (60 per grip): the right factor r is a non-zero multiple of a unit
      icosian; for the 12 retained elements the left factor is too; for the 48 jumble elements the left factor l has
      d(l v0, v0) = 2, lies in the branch of c, and fixes w_c = n_c v0;
  T4  every distinct pose of a given pose table has its right factor a multiple of a unit icosian;
  T5  every element of Lambda_0 maps the normal of every grip of the branch of grip 0 onto the normal of a grip of
      that branch (exact pole images).
usage: treefacts.py <research/jumbling> <pose table json> <out json>"""
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
import trees as T
from exact import Q5
t0 = time.time()
ctx = sim.get_context(); d = ctx.data
M = RS.menu('I_a')
tab = json.load(open(sys.argv[2])); OUT = sys.argv[3]
res = Counter()


def pkey(q):
    """Key of q up to a non-zero scalar of Q(sqrt5)."""
    for c in q:
        if not c.is_zero():
            return tuple(x / c for x in q)
    raise ValueError('zero quaternion')


UK = {pkey(u) for u in T.UNITS}
assert len(UK) == 60
N = [tuple(Q5(x) if not isinstance(x, Q5) else x for x in d.N[c]) for c in range(600)]
nb = []
for c in range(600):
    res['T1_dist_%d' % T.dist(N[c])] += 1
    nb.append(T.branch(N[c]))
res['T1_branch_sizes'] = 0
bs = Counter(nb)
for u in T.UNITS:
    res['T2_unit_dist_%d' % T.dist(u)] += 1
JI = set(RS.jumble_items(M))
for c in range(600):
    nc = N[c]; ncb = T.qconj(nc)
    for i in range(60):
        E = RS.element(M, c, i)
        l, r = T.factors(E)
        res['T3_right_in_2I' if pkey(r) in UK else 'T3_right_NOT_in_2I'] += 1
        if i in JI:
            ok = (T.dist(l) == 2 and T.branch(l) == nb[c] and T.dist(T.qmul(T.qmul(ncb, l), nc)) == 0)
            res['T3_jumble_left_ok' if ok else 'T3_jumble_left_BAD'] += 1
        else:
            res['T3_retained_left_in_2I' if pkey(l) in UK else 'T3_retained_left_NOT_in_2I'] += 1
    if c % 100 == 99:
        print(json.dumps({'grips': c + 1, 'sec': round(time.time() - t0, 1)}), flush=True)
for m in tab['poses']:
    l, r = T.factors(matrix_from_json(m))
    res['T4_pose_right_in_2I' if pkey(r) in UK else 'T4_pose_right_NOT_in_2I'] += 1
pole = {pkey(N[c]): c for c in range(600)}
br0 = [c for c in range(600) if nb[c] == nb[0]]
for i in range(60):
    E = RS.element(M, 0, i)
    for c2 in br0:
        v = tuple(sum((E[a][b] * N[c2][b] for b in range(4)), Q5(0)) for a in range(4))
        k = pole.get(pkey(v))
        res['T5_ok' if (k is not None and nb[k] == nb[0]) else 'T5_BAD'] += 1
out = {'counts': dict(sorted(res.items())), 'branch_sizes': {str(k): v for k, v in sorted(bs.items())},
       'table': Path(sys.argv[2]).name, 'table_digest': tab['digest'], 'distinct_poses': len(tab['poses']),
       'sec': round(time.time() - t0, 1)}
out['all_pass'] = (res['T1_dist_1'] == 600 and sorted(bs.values()) == [120] * 5 and res['T2_unit_dist_0'] == 120
                   and res['T3_right_in_2I'] == 36000 and res['T3_jumble_left_ok'] == 28800
                   and res['T3_retained_left_in_2I'] == 7200 and res['T4_pose_right_in_2I'] == len(tab['poses'])
                   and res['T5_ok'] == 60 * len(br0) and not any('BAD' in k or 'NOT' in k for k in res))
Path(OUT).write_text(json.dumps(out, indent=1) + '\n')
print(json.dumps(out))
