"""Certificate REFX-EXACT: every cap decision of certificate REFX, recomputed over Q(sqrt 5).

`refexact.py` decides in floating point, with margin 1e-7, whether the closed cap of a grip holds a posed region (inn)
or excludes it (out), and decides exactly only near ties (senior review round 13, T13-01). This script runs the same
closures (K+ orbits 1 to 10, hmax 40), checks that they reproduce certs/q1/refx-1-10.json (completeness, states,
classes, largest height, class digest), and then recomputes, for every class representative and all 600 grips, the
signs of n_c . y - kappa at the exact posed vertices y. A decision agrees when inn is "every sign >= 0" and out is
"every sign <= 0", as in the exact branch of refexact.py. It also reports the smallest float distance from 0 among the
decisions refexact.py took in floating point.
usage: refx_exact.py <research/jumbling> <out json>"""
import json, sys, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.dont_write_bytecode = True
import numpy as np
import refexact as RX

jdir, outp = sys.argv[1], sys.argv[2]
cert = json.loads((HERE.parent / 'certs/q1/refx-1-10.json').read_text())
C = RX.Ctx(jdir)
T = C.T
res = {}
t0 = time.time()
for o in range(1, 11):
    R = RX.explore(C, o, 40, 2000000)
    s = RX.summary(C, R, o)
    ref = cert[str(o)]
    same = all(s[k] == ref[k] for k in ('complete', 'states', 'classes', 'max_height', 'class_digest'))
    B = R.B
    bad_in = bad_out = 0
    gap = None
    for j, rep in enumerate(B.rep):
        ys = [T.qmul(rep, v) for v in B.V]
        inn, out = set(), set()
        for c in range(600):
            sg = [(C.dot(C.N[c], y) - C.KAP).sign() for y in ys]
            if min(sg) >= 0:
                inn.add(c)
            if max(sg) <= 0:
                out.add(c)
        bad_in += inn != set(B.caps[j])
        bad_out += out != set(B.outcaps[j])
        fv = np.array([[float(x) for x in y] for y in ys]) @ C.NF.T - C.KAPf
        ext = np.concatenate([fv.min(axis=0), fv.max(axis=0)])
        ext = np.abs(ext[np.abs(ext) > RX.Orbit.MARGIN])
        if ext.size:
            gap = float(ext.min()) if gap is None else min(gap, float(ext.min()))
    res[str(o)] = {'model_orbit': s['model_orbit'], 'closure_matches_refx': same, 'states': s['states'],
                   'classes': s['classes'], 'max_height': s['max_height'], 'cap_decisions': 2 * 600 * len(B.rep),
                   'inn_mismatch_classes': int(bad_in), 'out_mismatch_classes': int(bad_out),
                   'smallest_float_distance': round(gap, 9) if gap is not None else None}
    print(json.dumps({o: res[str(o)]}), flush=True)
summ = {'all_exact': all(r['closure_matches_refx'] and r['inn_mismatch_classes'] == 0 and r['out_mismatch_classes'] == 0
                         for r in res.values()),
        'classes': sum(r['classes'] for r in res.values()),
        'cap_decisions': sum(r['cap_decisions'] for r in res.values()),
        'smallest_float_distance': min(r['smallest_float_distance'] for r in res.values()),
        'sec': round(time.time() - t0, 1)}
print(json.dumps(summ), flush=True)
Path(outp).write_text(json.dumps({'orbits': res, 'summary': summ}, indent=1) + '\n')
