"""Return-pump search for one tracked piece p (menu I_a, exact simulator), from solved.

Alignment law (under H): a piece at left position x has a 3-face on the cut plane of grip c only if x is adjacent
to w_c in the tree; so a jumbled cap of branch b stays aligned with the cut planes of branch b only.
A pump cycle at (G, D, a, y), with branch(G) = branch(D) != branch(p), a in A4_D (a retained element), p inside
cap D and wholly outside cap G, a(p) inside cap G:
    (G, y^-1)   cap G goes out; p is not moved
    (D, a)      retained twist of the same branch: plane D is aligned with the jumbled content of cap G
    (G, y)      return: cap G content returns to the lattice, except what a carried in; p is raised by 2.
Also tried: a direct raise (jumble at an unstraddled cap of another branch that wholly holds p).
Depth-first over raises, children ordered by the number of off-lattice pieces (fewest first).
usage: pump2.py <research/jumbling> <piece> <max raises> <seed> <out json> [--per-level N] [--ys K]"""
import json, sys, time, random
from pathlib import Path
import numpy as np

J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
import restore as RS
import trees as T

t0 = time.time()
ctx = sim.get_context()
d, reg, kp = ctx.data, ctx.regions, ctx.kplus
M = RS.menu('I_a')
P = int(sys.argv[2]); MAXR = int(sys.argv[3]); SEED = int(sys.argv[4]); OUT = sys.argv[5]
PER = int(sys.argv[sys.argv.index('--per-level') + 1]) if '--per-level' in sys.argv else 4
NY = int(sys.argv[sys.argv.index('--ys') + 1]) if '--ys' in sys.argv else 4
rng = random.Random(SEED)
fm = lambda m: np.array([[float(v) for v in row] for row in m])
NF = np.array(d.NF, float); KAP = float(d.KAPPA)
orb = reg.orbit_of.astype(np.int64); tr = reg.transport.astype(np.int64)
V0 = np.array([[float(x) for x in v] for v in reg.rep_vertices[orb[P]]]) @ fm(kp.matrix(int(tr[P]))).T
JI = list(RS.jumble_items(M))
AI = [i for i in range(1, 60) if i not in JI]
EL, ELf = {}, {}


def el(c, i):
    k = (c, i)
    if k not in EL:
        EL[k] = RS.element(M, c, i)
        ELf[k] = fm(EL[k])
    return EL[k]


def inv(E):
    return tuple(tuple(E[j][i] for j in range(4)) for i in range(4))


gbr = []
for c in range(600):
    l, r = T.factors(el(c, JI[0]))
    gbr.append(T.branch(l))
st = sim.State(ctx, menu=M)


def lh():
    l, r = T.factors(st.pose(P))
    return T.dist(l), T.branch(l)


def apply(c, E):
    return st.apply(sim.Twist(ctx, c, E, 'menu', {'menu': M.name}), journal=True).applied


def posed():
    return V0 @ fm(st.pose(P)).T


best = {'h': 0}
stats = {'nodes': 0, 'direct': 0, 'pump_try': 0, 'G_blocked': 0, 'D_blocked': 0, 'ret_blocked': 0, 'pump_ok': 0}


def record(tag):
    h, b = lh()
    if h > best['h']:
        best['h'] = h
        print(json.dumps({'best_h': h, 'tag': tag, 'len': len(st.journal), 'off': st.off_lattice_count(),
                          'stats': stats, 'sec': round(time.time() - t0, 1)}), flush=True)
        Path(OUT).write_text(json.dumps({'piece': P, 'best_h': h, 'journal': st.journal, 'stats': stats}) + '\n')


def children():
    """yield (kind, letters) macro moves that raise p by 2; each letters list is applied in order."""
    h, b = lh()
    V = posed()
    vals = V @ NF.T - KAP
    lo, hi = vals.min(0), vals.max(0)
    hold = np.flatnonzero(lo > -1e-9).tolist()
    out = []
    # direct raises
    for c in hold:
        if h == 0 or gbr[c] != b:
            for y in rng.sample(JI, NY):
                out.append(('direct', [(c, y, False)]))
    # pumps
    for D in hold:
        if h > 0 and gbr[D] == b:
            continue
        for a in AI:
            W = V @ ELf[(D, a)].T if (D, a) in ELf else V @ fm(el(D, a)).T
            lo2 = (W @ NF.T - KAP).min(0)
            for G in np.flatnonzero(lo2 > -1e-9).tolist():
                if G == D or gbr[G] != gbr[D] or hi[G] > 1e-9:
                    continue
                for y in rng.sample(JI, NY):
                    out.append(('pump', [(G, y, True), (D, a, False), (G, y, False)]))
    rng.shuffle(out)
    return out


def dfs(depth):
    stats['nodes'] += 1
    if depth >= MAXR:
        return
    h0, _ = lh()
    tried = 0
    bad_G = set()
    for kind, letters in children():
        if tried >= PER:
            break
        if kind == 'pump' and letters[0][0] in bad_G:
            continue
        n = 0
        ok = True
        for k, (c, i, invert) in enumerate(letters):
            E = inv(el(c, i)) if invert else el(c, i)
            if not apply(c, E):
                ok = False
                if kind == 'pump':
                    stats[['G_blocked', 'D_blocked', 'ret_blocked'][k]] += 1
                    if k == 0:
                        bad_G.add(c)
                break
            n += 1
        if ok:
            h1, _ = lh()
            if h1 > h0:
                stats['direct' if kind == 'direct' else 'pump_ok'] += 1
                tried += 1
                record(kind)
                dfs(depth + 1)
        if kind == 'pump':
            stats['pump_try'] += 1
        for _ in range(n):
            st.undo()
        if best['h'] >= 2 * MAXR:
            return


record('start')
dfs(0)
print(json.dumps({'done': True, 'best_h': best['h'], 'stats': stats, 'sec': round(time.time() - t0, 1)}), flush=True)
