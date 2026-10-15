"""Return-pump search with small lenses, for one tracked piece p (menu I_a, exact simulator), from solved.

Pump cycle (G, y^-1) (D, a) (G, y): branch(G) = branch(D) != branch(p), a in A4_D, p wholly in cap D and wholly
outside cap G, a(p) wholly in cap G. The cohort (pieces carried into cap G by a, raised or twisted by the return)
is the content of cap D ∩ a^-1(cap G) minus cap G; the debris (jumbled pieces carried out of cap G) has the same
volume. Options are ordered by the float-estimated cohort size (fewest pieces first), with a float pre-test that no
current off-lattice piece straddles plane G or plane D. Every letter is checked by the exact simulator.
Direct raises (a jumble at an unstraddled cap of another branch holding p) are offered after the pumps.
usage: pump3.py <research/jumbling> <piece> <max raises> <seed> <out json> [--per-level N] [--ys K] [--maxlens L]"""
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
arg = lambda k, dv: (type(dv))(sys.argv[sys.argv.index(k) + 1]) if k in sys.argv else dv
PER = arg('--per-level', 4); NY = arg('--ys', 2); MAXL = arg('--maxlens', 400)
rng = random.Random(SEED)
fm = lambda m: np.array([[float(v) for v in row] for row in m])
NF = np.array(d.NF, float); KAP = float(d.KAPPA)
NP = len(reg.orbit_of)
orb = reg.orbit_of.astype(np.int64); tr = reg.transport.astype(np.int64)
Kf = np.array([fm(kp.matrix(i)) for i in range(7200)])
repV = [np.array([[float(x) for x in v] for v in vs]) for vs in reg.rep_vertices]
HV = [repV[orb[q]] @ Kf[tr[q]].T for q in range(NP)]          # home vertices
nv = np.array([len(v) for v in HV]); off_ = np.concatenate([[0], np.cumsum(nv)])
HVall = np.concatenate(HV)                                     # (sum nv, 4)
owner = np.repeat(np.arange(NP), nv)
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
gbr = np.array(gbr)
st = sim.State(ctx, menu=M)
pf_cache = {}


def posed_all():
    """posed vertices of all pieces (float), via pose ids."""
    pid = np.asarray(st.pose_id)
    U = np.unique(pid)
    Pm = np.zeros((int(U.max()) + 1, 4, 4))
    for i in U.tolist():
        if i not in pf_cache:
            pf_cache[i] = fm(st._pose_matrix(i))
        Pm[i] = pf_cache[i]
    Pv = Pm[pid[owner]]                                        # (sum nv, 4, 4)
    return np.einsum('nij,nj->ni', Pv, HVall), pid


def lh(q=P):
    l, r = T.factors(st.pose(q))
    return T.dist(l), T.branch(l)


def apply(c, E):
    return st.apply(sim.Twist(ctx, c, E, 'menu', {'menu': M.name}), journal=True).applied


def minmax(vals):
    lo = np.minimum.reduceat(vals, off_[:-1], axis=0)
    hi = np.maximum.reduceat(vals, off_[:-1], axis=0)
    return lo, hi


best = {'h': 0}
stats = {'nodes': 0, 'direct': 0, 'pump_try': 0, 'G_blocked': 0, 'D_blocked': 0, 'ret_blocked': 0, 'pump_ok': 0,
         'pre_rejected': 0}


def record(tag, extra=None):
    h, b = lh()
    if h > best['h']:
        best['h'] = h
        info = {'best_h': h, 'tag': tag, 'len': len(st.journal), 'off': st.off_lattice_count(), 'stats': stats,
                'sec': round(time.time() - t0, 1)}
        if extra:
            info.update(extra)
        print(json.dumps(info), flush=True)
        Path(OUT).write_text(json.dumps({'piece': P, 'best_h': h, 'journal': st.journal, 'stats': stats}) + '\n')


def options():
    h, b = lh()
    X, pid = posed_all()
    Vp = X[off_[P]:off_[P + 1]]
    valsp = Vp @ NF.T - KAP
    hold = np.flatnonzero(valsp.min(0) > -1e-9)
    outside = valsp.max(0) < 1e-9
    cand = []
    for D in hold.tolist():
        if h > 0 and gbr[D] == b:
            continue
        for a in AI:
            W = Vp @ (ELf[(D, a)] if (D, a) in ELf else fm(el(D, a))).T
            lo2 = (W @ NF.T - KAP).min(0)
            for G in np.flatnonzero(lo2 > -1e-9).tolist():
                if G != D and gbr[G] == gbr[D] and outside[G]:
                    cand.append((D, a, G))
    if not cand:
        return [], hold
    grips = sorted({c for D, a, G in cand for c in (D, G)})
    gi = {c: k for k, c in enumerate(grips)}
    vals = X @ NF[grips].T - KAP
    lo, hi = minmax(vals)
    strad = (lo < -1e-9) & (hi > 1e-9)                         # pieces straddling each plane now
    inD = lo > -1e-9
    out = []
    for D, a, G in cand:
        if strad[:, gi[G]].any() or strad[:, gi[D]].any():
            stats['pre_rejected'] += 1
            continue
        qs = np.flatnonzero(inD[:, gi[D]] & ~(lo[:, gi[G]] > -1e-9))
        if len(qs) == 0:
            continue
        idx = np.concatenate([np.arange(off_[q], off_[q + 1]) for q in qs.tolist()])
        W = X[idx] @ ELf[(D, a)].T
        v = W @ NF[G] - KAP
        lo_g = np.minimum.reduceat(v, np.concatenate([[0], np.cumsum(nv[qs])[:-1]]))
        lens = int((lo_g > -1e-9).sum())
        if lens <= MAXL:
            out.append((lens, D, a, G))
    out.sort()
    return out, hold


def dfs(depth):
    stats['nodes'] += 1
    if depth >= MAXR:
        return
    h0, b0 = lh()
    opts, hold = options()
    tried = 0
    bad_G = set()
    for lens, D, a, G in opts:
        if tried >= PER:
            break
        if G in bad_G:
            continue
        for y in rng.sample(JI, NY):
            stats['pump_try'] += 1
            if not apply(G, inv(el(G, y))):
                stats['G_blocked'] += 1; bad_G.add(G); break
            if not apply(D, el(D, a)):
                stats['D_blocked'] += 1; st.undo(); break
            if not apply(G, el(G, y)):
                stats['ret_blocked'] += 1; st.undo(); st.undo(); break
            h1, _ = lh()
            if h1 > h0:
                stats['pump_ok'] += 1; tried += 1
                record('pump', {'lens': lens, 'D': D, 'a': a, 'G': G, 'y': y})
                dfs(depth + 1)
            st.undo(); st.undo(); st.undo()
            if best['h'] >= 2 * MAXR:
                return
    # direct raises as a fallback
    if tried == 0:
        for c in hold.tolist():
            if h0 > 0 and gbr[c] == b0:
                continue
            for y in rng.sample(JI, NY):
                if apply(c, el(c, y)):
                    h1, _ = lh()
                    if h1 > h0:
                        stats['direct'] += 1; tried += 1
                        record('direct', {'c': c, 'y': y})
                        dfs(depth + 1)
                    st.undo()
                else:
                    break
            if tried >= PER:
                break


record('start')
print(json.dumps({'piece': P, 'orbit': int(d.orbit_id[P]), 'sec': round(time.time() - t0, 1)}), flush=True)
dfs(0)
print(json.dumps({'done': True, 'best_h': best['h'], 'stats': stats, 'sec': round(time.time() - t0, 1)}), flush=True)
