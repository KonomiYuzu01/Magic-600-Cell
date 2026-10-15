"""Inner exclusion check (menu I_a): from solved, one jumble at grip c; for every other grip T, does some ball piece
(K+ orbits 0..4, whose home regions meet the open inscribed ball) straddle plane T (float, margin 1e-9)? Tabulated by
the angle between the grips and by same/different tree branch; exact classify for the admissibility of T.
usage: innerx.py <research/jumbling> <grip> <out json>"""
import json, sys, time
from collections import Counter, defaultdict
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
ctx = sim.get_context(); d, reg, kp = ctx.data, ctx.regions, ctx.kplus
M = RS.menu('I_a')
C = int(sys.argv[2]); OUT = sys.argv[3]
fm = lambda m: np.array([[float(v) for v in row] for row in m])
NF = np.array(d.NF, float); KAP = float(d.KAPPA); RIN = float(d.NN) ** 0.5
orb = reg.orbit_of.astype(np.int64); tr = reg.transport.astype(np.int64)
JI = list(RS.jumble_items(M))
gbr = [T.branch(T.factors(RS.element(M, c, JI[0]))[0]) for c in range(600)]
ball = [o for o in range(len(reg.rep_vertices)) if o <= 4]
st = sim.State(ctx, menu=M)
E = RS.element(M, C, JI[0])
assert st.apply(sim.Twist(ctx, C, E, 'menu', {'menu': M.name}), journal=True).applied
Ef = fm(E)
nh = NF / np.linalg.norm(NF, axis=1)[:, None]
ang = np.degrees(np.arccos(np.clip(nh @ nh[C], -1, 1)))
qs = np.flatnonzero(np.isin(orb, ball))
inner_strad = np.zeros(600, bool)
for q in qs.tolist():
    V = np.array([[float(x) for x in v] for v in reg.rep_vertices[orb[q]]]) @ fm(kp.matrix(int(tr[q]))).T
    if (V @ NF[C] - KAP).min() > -1e-9:          # inside cap C: moved by the jumble
        V = V @ Ef.T
    vals = V @ NF.T - KAP
    inner_strad |= (vals.min(0) < -1e-9) & (vals.max(0) > 1e-9)
tab = defaultdict(Counter)
for Tg in range(600):
    if Tg == C or ang[Tg] > 60.5:
        continue
    cl = st.classify(Tg, need_inside=False, stop_at_straddle=True)
    key = '%.2f %s' % (ang[Tg], 'same' if gbr[Tg] == gbr[C] else 'diff')
    tab[key]['n'] += 1
    tab[key]['blocked'] += cl.straddle is not None
    tab[key]['ball_straddles'] += bool(inner_strad[Tg])
out = {'grip': C, 'jumble_item': JI[0], 'by_angle': {k: dict(v) for k, v in sorted(tab.items())}, 'sec': round(time.time() - t0, 1)}
Path(OUT).write_text(json.dumps(out, indent=1) + '\n')
print(json.dumps(out, indent=0))
