"""Certificate G': for grip 0, every jumble x in Lambda_0 and every home h of the listed orbits in cap 0, the set of
grips d != 0 whose closed cap {n_d . y >= kappa} wholly contains x(A_h). Exact Q(sqrt5) evaluation of every vertex
whenever the float value is within 1e-6 of kappa. Output per orbit: containing grips with their angle to grip 0
and whether they share an orbit-0 home with cap 0."""
import json, sys
from pathlib import Path
from collections import Counter, defaultdict
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
import restore as RS
from exact import matvec, dot
ctx = sim.get_context()
d = ctx.data
reg = ctx.regions
M = RS.menu('I_a')
JI = RS.jumble_items(M)
orbits = [int(x) for x in sys.argv[2].split(',')]
NF = np.array([[float(v) for v in row] for row in d.N])
ang = np.degrees(np.arccos(np.clip(NF @ NF[0] / float(d.NN), -1, 1)))
kap = d.KAPPA; kapf = float(kap)
members = [int(p) for p in d.cap_members(0)]
o0share = set()
for p in members:
    if int(d.orbit_id[p]) == 0:
        o0share |= {c for c in d.signature_list(p) if c != 0}
res = {}
exact_evals = 0
for o in orbits:
    hs = [p for p in members if int(d.orbit_id[p]) == o]
    cont = Counter()
    for h in hs:
        V = reg.vertices(h)
        for i in JI:
            X = RS.element(M, 0, i)
            W = [matvec(X, v) for v in V]
            Wf = np.array([[float(t) for t in w] for w in W])
            H = Wf @ NF.T - kapf          # vertices x grips
            mins = H.min(axis=0)
            for c in np.flatnonzero(mins > -1e-6).tolist():
                if c == 0:
                    continue
                if mins[c] > 1e-6:
                    ok = True
                else:
                    exact_evals += 1
                    ok = all((dot(d.N[c], w) - kap).sign() >= 0 for w in W)
                if ok:
                    cont[c] += 1
    res[o] = {'homes_in_cap0': len(hs), 'containing': {str(c): {'count': n, 'angle': round(float(ang[c]), 2),
              'shares_orbit0_home': c in o0share} for c, n in sorted(cont.items())}}
    print(o, len(hs), {c: (n, round(float(ang[c]), 2), c in o0share) for c, n in sorted(cont.items())}, flush=True)
print('exact evaluations', exact_evals)
json.dump({'grip': 0, 'orbit0_sharing_grips': sorted(o0share), 'orbits': res}, open(sys.argv[3], 'w'), indent=1)
