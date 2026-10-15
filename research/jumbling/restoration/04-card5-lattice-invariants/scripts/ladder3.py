"""Height ladder by raise-and-refill. Base: a word U from solved with exactly one off piece p (left height h).
Step: y = a jumble at a grip f whose cap wholly holds p, class(f) != outer class of p (raises p to h+2).
With C = U y, a retained twist (g,a) whose cap holds no off piece and refills exactly one hole h1 (refill set
R(g,a) = {h in H_g : a^-1(h) not in H} of size one) gives W = C (g,a) C^-1, whose only off piece is the
refilling piece z, which travels backwards along the trajectory of the vacancy h1 (every other piece repeats
the exact reverse of C; the vacated slot cannot straddle). z's path is therefore screened alone with floating
point vertex tests (inside / outside / straddle) and an exact pose product; promising candidates (z ends off
the lattice at height >= h+2) are replayed in full from solved before they are accepted.
usage: ladder3.py <research/jumbling> <classes.json> <base json> <out json> <budget s> <steps> <seed>"""
import json, sys, time, random
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim, trees
import restore as RS
from exact import transpose, matmul
from sim.kplus import to_tuple
ctx = sim.get_context()
d, reg, kp = ctx.data, ctx.regions, ctx.kplus
M = RS.menu('I_a')
cls = {int(k): v for k, v in json.load(open(sys.argv[2])).items()}
base = json.load(open(sys.argv[3]))
OUT, budget, steps, seed = sys.argv[4], float(sys.argv[5]), int(sys.argv[6]), int(sys.argv[7])
rng = random.Random(seed)
JI = list(RS.jumble_items(M)); ident = RS.element(M, 0, 0)
RI = [i for i in range(len(M.items)) if i not in JI and RS.element(M, 0, i) != ident]
cb = {}
for c in range(600):
    if cls[c] not in cb:
        cb[cls[c]] = trees.pose_tree(RS.element(M, c, JI[0]))[2]
bclass = {v: k for k, v in cb.items()}
fm = lambda m: np.array([[float(v) for v in row] for row in m])
NF = np.array(d.NF, float); KAP = float(d.KAPPA); TOL = 1e-7
Kf = {}
def kf(i):
    if i not in Kf:
        Kf[i] = fm(kp.matrix(i))
    return Kf[i]
repV = [np.array([[float(v) for v in vert] for vert in vs]) for vs in reg.rep_vertices]
inv_cache, TW, KI, FM = {}, {}, {}, {}
def inv(l):
    if l not in inv_cache:
        c, i = l
        T = to_tuple(transpose(RS.element(M, c, i)))
        inv_cache[l] = (c, next(j for j in range(len(M.items)) if RS.element(M, c, j) == T))
    return inv_cache[l]
winv = lambda w: [inv(l) for l in reversed(w)]
def tw(l):
    if l not in TW:
        TW[l] = sim.Twist(ctx, l[0], RS.element(M, l[0], l[1]), 'menu', {'menu': M.name})
    return TW[l]
def kidx(l):
    if l not in KI:
        KI[l] = int(kp.index_of_matrix(RS.element(M, l[0], l[1])))
    return KI[l]
def fmat(l):
    if l not in FM:
        FM[l] = fm(RS.element(M, l[0], l[1]))
    return FM[l]
def images(k, slots):
    s = np.asarray(slots, np.int64)
    return reg.piece_of[reg.orbit_of[s], kp.compose_vec(np.full(len(s), k), reg.transport[s])]
def holes_and_occ(st):
    K = st._kpose()[st.pose_id]
    lat = np.flatnonzero(K >= 0)
    comp = kp.compose_vec(K[lat], reg.transport[lat])
    dest = reg.piece_of[reg.orbit_of[lat], comp]
    occ = -np.ones(len(K), np.int64); occ[dest] = lat
    return np.flatnonzero(occ < 0), occ, K
def height(pose):
    dl, dr, bl, br = trees.pose_tree(pose)
    return dl, bclass.get(bl)
def trajectory(z, k0, word):
    """z starts on the lattice with K+ pose index k0; follow it alone through word. Returns (ok, exact pose)."""
    V = repV[int(reg.orbit_of[z])] @ kf(int(kp.compose_vec(np.array([k0]), np.array([reg.transport[z]]))[0])).T
    pose = kp.matrix(k0)
    for l in word:
        vals = V @ NF[l[0]] - KAP
        if (vals > -TOL).all() and (vals > TOL).any():
            V = V @ fmat(l).T
            pose = to_tuple(matmul(RS.element(M, l[0], l[1]), pose))
        elif not (vals < TOL).all():
            return False, None
    return True, pose
st = sim.State(ctx, menu=M)
U = [tuple(l) for l in base['word']]
for l in U:
    assert st.apply(tw(l)).applied
t0 = time.time()
res = {'steps': []}
for step in range(steps):
    off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
    assert len(off) == 1
    p = int(off[0])
    h0, outer = height(st.pose(p))
    print('step', step, 'piece', p, 'orbit', int(d.orbit_id[p]), 'height', h0, 'outer', outer, 'length', len(U), flush=True)
    fs = []
    for f in range(600):
        if cls[f] == outer:
            continue
        cl = st.classify(f, need_inside=True, stop_at_straddle=True)
        if cl.straddle is None and not cl.uncertain and p in set(cl.inside.tolist()):
            fs.append(f)
    print('  catching grips', fs, '%.0fs' % (time.time() - t0), flush=True)
    found = None
    screened = 0
    cands = [(f, yi) for f in fs for yi in JI]
    rng.shuffle(cands)
    for f, yi in cands:
        if found or time.time() - t0 > budget:
            break
        y = (f, yi)
        assert st.apply(tw(y)).applied
        C = U + [y]
        Cinv = winv(C)
        H, occ, K = holes_and_occ(st)
        Hs = set(H.tolist())
        offs = set(np.flatnonzero(K < 0).tolist())
        cnt = {}
        for h in H.tolist():
            for g in d.signature_list(int(h)):
                cnt.setdefault(int(g), []).append(int(h))
        clean = {}
        best = []
        for g, hs in cnt.items():
            for a in RI:
                pre = images(kidx(inv((g, a))), hs)
                R = [(h, q) for h, q in zip(hs, pre.tolist()) if q not in Hs]
                if len(R) != 1:
                    continue
                if g not in clean:
                    cl = st.classify(g, need_inside=True, stop_at_straddle=True)
                    clean[g] = cl.straddle is None and not cl.uncertain and not (set(cl.inside.tolist()) & offs)
                if not clean[g]:
                    break
                h1, src = R[0]
                z = int(occ[src])
                k0 = int(kp.compose_vec(np.array([kidx((g, a))]), np.array([K[z]]))[0])
                ok, pose = trajectory(z, k0, Cinv)
                screened += 1
                if not ok:
                    continue
                hz, oz = height(pose)
                if hz >= h0 + 2:
                    best.append((hz, g, a, z))
        best.sort(reverse=True)
        for hz, g, a, z in best[:3]:
            W = C + [(g, a)] + Cinv
            st2 = sim.State(ctx, menu=M)
            okw = all(st2.apply(tw(l)).applied for l in W)
            if not okw:
                print('  replay inadmissible', f, yi, g, a, flush=True)
                continue
            off2 = np.flatnonzero(st2._kpose()[st2.pose_id] < 0)
            if len(off2) == 1 and int(off2[0]) == z and height(st2.pose(z))[0] == hz:
                found = (W, z, hz)
                st = st2
                break
            print('  replay mismatch', f, yi, g, a, off2.tolist()[:5], flush=True)
        if not found:
            assert st.apply(tw(inv(y))).applied
    print('  screened', screened, 'found', None if not found else (found[1], found[2], len(found[0])), '%.0fs' % (time.time() - t0), flush=True)
    if not found:
        break
    U = found[0]
    res['steps'].append({'piece': found[1], 'orbit': int(d.orbit_id[found[1]]), 'height': found[2], 'length': len(U)})
    res['word'] = [list(l) for l in U]
    json.dump(res, open(OUT, 'w'))
print('done', [(s['height'], s['length']) for s in res['steps']])
