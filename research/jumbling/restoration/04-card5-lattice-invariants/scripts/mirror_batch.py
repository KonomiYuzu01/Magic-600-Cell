"""Mirror every single-hole word of a singles jsonl into a pure position-sign generator b_o.
For a single v (one off piece, one hole h in orbit o), a grip d whose cap holds h and a retained half-turn a at d:
  w = v v^a (d,a) (v v^a)^-1,  v^a = v conjugated letter by letter by the global rotation a (menu-equivariant).
U = v v^a leaves two holes h, a(h) of orbit o that (d,a) swaps; the frame parts cancel because a^2 = 1, so the
vacancy formula predicts nu(w) = b_o. Every candidate is replayed from solved in the exact simulator; nu is read
from the lattice stickers. Stops after `per` verified successes per hole orbit.
usage: mirror_batch.py <research/jumbling> <repo root> <i23 scripts> <singles jsonl> <out jsonl> [per]"""
import json, sys, time
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
sys.path.insert(0, sys.argv[3])
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import restore as RS
import sim
from exact import matmul, transpose
from sim.kplus import to_tuple
from nucoords import NuCoords
ctx = RS.ctx
d, kp = ctx.data, ctx.kplus
M = RS.menu('I_a')
NC = NuCoords(sys.argv[2])
per = int(sys.argv[6]) if len(sys.argv) > 6 else 1
NI = len(M.items)
item_of = {}
def item(c, A):
    if c not in item_of:
        item_of[c] = {RS.element(M, c, j): j for j in range(NI)}
    return item_of[c][to_tuple(A)]
def inv_letter(l):
    c, j = l
    return (c, item(c, transpose(RS.element(M, c, j))))
TW = {}
def tw(l):
    if l not in TW:
        TW[l] = sim.Twist(ctx, l[0], RS.element(M, l[0], l[1]), 'menu', {'menu': M.name})
    return TW[l]
ident0 = RS.element(M, 0, 0)
stab0 = [kp.matrix(s) for s in kp.stab0]
RI = [i for i in range(NI) if RS.element(M, 0, i) in stab0 and RS.element(M, 0, i) != ident0]
rows = [json.loads(x) for x in open(sys.argv[4]) if x.strip()]
rows.sort(key=lambda r: r['length'])
done = {}
fout = open(sys.argv[5], 'a')
t0 = time.time()
for r in rows:
    if len(r['holes']) != 1:
        continue
    o, h = r['hole_orbit'][0], r['holes'][0]
    if done.get(o, 0) >= per:
        continue
    v = [tuple(x) for x in r['word']]
    for dd in d.signature_list(h):
        if done.get(o, 0) >= per:
            break
        for i in RI:
            A = RS.element(M, dd, i)
            if to_tuple(matmul(A, A)) != RS.element(M, dd, 0):
                continue
            perm = kp.perms[kp.index_of_matrix(A)]
            va = [(int(perm[c]), item(int(perm[c]), matmul(matmul(A, RS.element(M, c, j)), transpose(A)))) for c, j in v]
            U = v + va
            w = U + [(dd, i)] + [inv_letter(l) for l in reversed(U)]
            st = sim.State(ctx, menu=M)
            n = 0
            for l in w:
                if not st.apply(tw(l)).applied:
                    break
                n += 1
            ok = n == len(w)
            lat = ok and bool(st.is_lattice())
            nu = None
            if lat:
                nu = {str(k): list(x) for k, x in sorted(NC.nu(np.asarray(st.lattice_stickers()['labels'], np.int64)).items())}
            print('hole orbit', o, 'd', dd, 'a', i, 'len', len(w), 'admissible', ok, 'lattice', lat, 'nu', nu,
                  '%.0fs' % (time.time() - t0), flush=True)
            if lat and nu == {str(o): [1, 0]}:
                fout.write(json.dumps({'orbit': o, 'nu': nu, 'length': len(w), 'single_length': len(v), 'hole': h,
                                       'd': dd, 'a_item': i, 'word': [list(l) for l in w]}) + '\n')
                fout.flush()
                done[o] = done.get(o, 0) + 1
                break
print('done', dict(sorted(done.items())), '%.0fs' % (time.time() - t0))
