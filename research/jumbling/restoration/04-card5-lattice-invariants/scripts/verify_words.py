"""Certificate CERT-LB: replay every generator word of A_J from solved and compute the span of its nu values.

For each row of the words file (fields: name, source, letters = [[grip, I_a menu item], ...] or records = exact
twist records, claimed = nu as {orbit: [sign, frame]}):
  1. every twist is an element of the I_a menu at its grip (records: Menu.contains; letters: by construction from
     the menu item), and every twist is admissible at its prefix, starting from the solved configuration;
  2. the end configuration is on the lattice (every pose in K+);
  3. nu is computed with the coherent-frame coordinates (nucoords.NuCoords) and compared with the claim;
  4. independently, the 35 position signs are recomputed from the final poses only: every final pose must be an
     exact rotation permuting the 600 poles, each piece is mapped to the piece whose home signature is the pole
     image of its own, and the parity of that permutation is taken on each model orbit_id class;
  5. the word is checked to be a conjugated half-turn u (d, a) u^-1 or not: odd length, middle letter a retained
     twist whose square is the identity, and letter L-1-i the inverse (transpose) of letter i at the same grip.
Then the F2 rank over the 43 C2 coordinates (35 signs b0..b34 and the C2 frames f0 f3 f6 f8 f11 f22 f25 f32), the
C5 coordinates (17, 28), the untouched coordinates and the relations satisfied by every vector are reported.
usage: verify_words.py <research/jumbling> <repo root> <nucoords dir> <words jsonl> <out json>"""
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
from exact import matvec, Q5, matmul, transpose, identity
from sim.kplus import to_tuple
from sim.twists import Twist
from nucoords import NuCoords
T0 = time.time()
ctx = RS.ctx
d = ctx.data
M = RS.menu('I_a')
assert M.identity == 'fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c', M.identity
NC = NuCoords(sys.argv[2])
mo, mv = d.mask_offsets, d.mask_values
sig_index = {tuple(mv[mo[p]:mo[p + 1]].tolist()): p for p in range(len(mo) - 1)}
oid = np.asarray(d.orbit_id)
POLE = {tuple(n): i for i, n in enumerate(d.N)}
C2F = [0, 3, 6, 8, 11, 22, 25, 32]
names = [f'b{o}' for o in range(35)] + [f'f{o}' for o in C2F]
def pole_perm(m):
    out = np.empty(600, np.int64)
    for c in range(600):
        img = tuple(matvec(m, d.N[c]))
        if img not in POLE:
            return None
        out[c] = POLE[img]
    return out
def parity(perm):
    seen = np.zeros(len(perm), bool); odd = 0
    for i in range(len(perm)):
        if seen[i]:
            continue
        j, L = i, 0
        while not seen[j]:
            seen[j] = True; j = perm[j]; L += 1
        odd ^= (L - 1) & 1
    return odd
def signs_from_poses(st):
    perm_of = {}
    for pid in np.unique(st.pose_id).tolist():
        pp = pole_perm(st._pose_matrix(pid))
        assert pp is not None, 'a final pose does not permute the poles'
        perm_of[pid] = pp
    NPc = len(mo) - 1
    dest = np.arange(NPc)
    for p in np.flatnonzero(st.pose_id != 0).tolist():
        pp = perm_of[int(st.pose_id[p])]
        q = sig_index.get(tuple(sorted(pp[mv[mo[p]:mo[p + 1]]].tolist())))
        assert q is not None, 'image signature is not a piece'
        dest[p] = q
    assert len(np.unique(dest)) == NPc, 'position map is not a bijection'
    assert np.all(oid[dest] == oid), 'a piece changed orbit class'
    out = {}
    for o in range(35):
        idx = np.flatnonzero(oid == o)
        loc = {int(p): i for i, p in enumerate(idx)}
        out[o] = parity(np.array([loc[int(dest[p])] for p in idx], np.int64))
    return out
TW = {}
def twist_of_letter(c, j):
    if (c, j) not in TW:
        TW[(c, j)] = sim.Twist(ctx, c, RS.element(M, c, j), 'menu', {'menu': M.name})
    return TW[(c, j)]
ID4 = to_tuple(identity(4))
def conj_half_turn(tws):
    L = len(tws)
    if L % 2 == 0:
        return False
    m = tws[L // 2]
    if m.kidx < 0 or to_tuple(matmul(m.matrix, m.matrix)) != ID4:
        return False
    for i in range(L // 2):
        a, b = tws[i], tws[L - 1 - i]
        if a.grip != b.grip or to_tuple(b.matrix) != to_tuple(transpose(a.matrix)):
            return False
    return True
rows = [json.loads(x) for x in open(sys.argv[4]) if x.strip()]
conj_vecs = []
report = []
vecs = []
c5 = {}
for r in rows:
    t1 = time.time()
    st = sim.State(ctx, menu=M)
    if 'letters' in r:
        tws = [twist_of_letter(int(c), int(j)) for c, j in r['letters']]
    else:
        tws = [Twist.from_record(ctx, rec) for rec in r['records']]
        assert all(M.contains(t) for t in tws), r['name'] + ': a twist is not in the I_a menu'
    cht = conj_half_turn(tws)
    ok = True
    for i, t in enumerate(tws):
        out = st.apply(t)
        if not out.applied:
            ok = False
            print(r['name'], 'inadmissible at step', i, out.status, flush=True)
            break
    lat = ok and bool(st.is_lattice())
    nu = None; agree = False; signs_ok = None
    if lat:
        nu = {str(k): [int(x[0]), int(x[1])] for k, x in sorted(NC.nu(np.asarray(st.lattice_stickers()['labels'], np.int64)).items())}
        agree = nu == {str(k): list(v) for k, v in sorted(r['claimed'].items(), key=lambda kv: int(kv[0]))}
        sp = signs_from_poses(st)
        signs_ok = all(sp[o] == (nu.get(str(o), [0, 0])[0] & 1) for o in range(35))
        v = [0] * 43
        for o, (s, f) in nu.items():
            o = int(o)
            v[o] = s & 1
            if o in C2F:
                v[35 + C2F.index(o)] = f & 1
            if o in (17, 28) and f % 5:
                c5.setdefault(o, []).append(r['name'])
        vecs.append((r['name'], v))
        if cht:
            conj_vecs.append(v)
    rep = {'name': r['name'], 'source': r.get('source'), 'length': len(tws), 'admissible_every_prefix': ok,
           'end_on_lattice': lat, 'conjugated_half_turn': cht, 'nu': nu, 'nu_agrees_with_claim': agree, 'signs_from_poses_agree': signs_ok,
           'seconds': round(time.time() - t1, 1)}
    report.append(rep)
    print(json.dumps(rep), flush=True)
def f2rank(rows_):
    basis = {}
    for v in rows_:
        x = int(''.join(str(b) for b in reversed(v)), 2)
        while x:
            p = x.bit_length() - 1
            if p in basis:
                x ^= basis[p]
            else:
                basis[p] = x
                break
    return len(basis)
A = np.array([v for _, v in vecs], dtype=np.uint8)
R = A.copy(); rank = 0
for c in range(43):
    p = next((i for i in range(rank, len(R)) if R[i, c]), None)
    if p is None:
        continue
    R[[rank, p]] = R[[p, rank]]
    for i in range(len(R)):
        if i != rank and R[i, c]:
            R[i] ^= R[rank]
    rank += 1
touched = sorted(set(np.flatnonzero(A.any(0)).tolist()))
M2 = A[:, touched].copy(); Rm = M2.copy(); pv = []; rr = 0
for c in range(M2.shape[1]):
    p = next((i for i in range(rr, len(Rm)) if Rm[i, c]), None)
    if p is None:
        continue
    Rm[[rr, p]] = Rm[[p, rr]]
    for i in range(len(Rm)):
        if i != rr and Rm[i, c]:
            Rm[i] ^= Rm[rr]
    pv.append(c); rr += 1
rels = []
for f in [c for c in range(M2.shape[1]) if c not in pv]:
    y = np.zeros(M2.shape[1], np.uint8); y[f] = 1
    for i, c in enumerate(pv):
        y[c] = Rm[i, f]
    rels.append('+'.join(names[touched[c]] for c in range(M2.shape[1]) if y[c]))
summary = {'words': len(rows), 'all_admissible': all(x['admissible_every_prefix'] for x in report),
           'all_on_lattice': all(x['end_on_lattice'] for x in report),
           'all_nu_agree': all(x['nu_agrees_with_claim'] for x in report),
           'all_signs_from_poses_agree': all(x['signs_from_poses_agree'] for x in report),
           'rank_C2': rank, 'C5_nonzero': c5, 'conj_half_turn_words': len(conj_vecs),
           'conj_half_turn_rank': f2rank(conj_vecs),
           'conj_half_turn_touched': [names[c] for c in range(43) if any(v[c] for v in conj_vecs)], 'untouched_C2': [names[c] for c in range(43) if c not in touched],
           'relations_on_touched': rels, 'menu_identity': M.identity, 'model_identity': d.identity,
           'seconds': round(time.time() - T0, 1)}
print(json.dumps(summary), flush=True)
Path(sys.argv[5]).write_text(json.dumps({'summary': summary, 'words': report}, indent=1))
