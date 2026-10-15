"""Audit 'shrink': exhaustive single-letter screen at E2 (own code, not singleP.py).

For every grip c (0..599): each piece of B' is classified exactly against the cut of c (own per-piece kernel calls on
the exact common pose m), and the simulator's own classifier decides admissibility (any straddle at E2). For every
grip whose cut holds (inside) at least one piece of B', and every menu item i whose element at c is not the
identity, record whether the letter would put all of B' on K: every B' piece inside and E(c,i) m in K+. The set of
admissible letters that do so is compared with singleP.json; each one is applied at E2 (simulator, admissible) and
its parked set counted and compared with the lattice pieces inside the cut at E2; then undone.
Grips where B' straddles but no piece is inside are also screened (as inadmissible letters) for completeness.

Usage: python3 -I -B single.py <out json>
"""
import json
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
S = Path('<work>')
TH = S / 'wt-e3/research/jumbling/theory'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))

import numpy as np  # noqa: E402

import blocks as BL  # noqa: E402
from exact import matmul  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

O, CL, restore = BL.O, BL.CL, BL.restore
kp, reg, data = CL.ctx.kplus, CL.ctx.regions, CL.ctx.data
T0 = time.time()


def main():
    out = Path(sys.argv[1])
    R, M = O.start(str(S / 'v26/B4/E2-table.json'))
    st = R.st
    d0 = st.digest()
    K0 = st._kpose()[st.pose_id]
    off = np.flatnonzero(K0 < 0)
    lat0 = K0 >= 0
    m = st.pose(int(off[0]))
    assert all(st.pose(int(p)) == m for p in off)
    side = {}
    for c in range(600):
        ev = Evaluator(pullback_form(m, data.N[c]))
        side[c] = [ev.classify(reg.vform(int(p)))[0] for p in off]
    adm = {}
    for c in range(600):
        cl = st.classify(c, need_inside=True, stop_at_straddle=True)
        assert not cl.uncertain
        adm[c] = cl.straddle is None
        # consistency: simulator straddle at E2 iff some B' piece straddles (lattice pieces never straddle)
        assert adm[c] == ('straddle' not in side[c]), c
    grips_any_in = [c for c in range(600) if 'in' in side[c]]
    grips_all_in = [c for c in range(600) if all(s == 'in' for s in side[c])]
    grips_strad_only = [c for c in range(600) if 'straddle' in side[c] and 'in' not in side[c]]
    rows = []
    hits = []
    for c in grips_any_in + grips_strad_only:
        for i in range(len(M.items)):
            e = restore.element(M, c, i)
            if e == restore.I4:
                continue
            em = kp.index_of_matrix(to_tuple(matmul(e, m)))
            ek = kp.index_of_matrix(e)
            nin = sum(s == 'in' for s in side[c])
            puts = nin == len(off) and em >= 0
            rows.append({'grip': c, 'item': i, 'admissible': adm[c], 'n_in': nin, 'e_in_K': ek >= 0,
                         'em_in_K': em >= 0, 'puts_Bprime_on_K': bool(puts)})
            if puts and adm[c]:
                hits.append((c, i))
    # apply each hit at E2
    applied = []
    for c, i in hits:
        cl = st.classify(c, need_inside=True, stop_at_straddle=False)
        ins = np.asarray(cl.inside, np.int64)
        lat_in = np.sort(ins[lat0[ins]])
        R.apply(c, i)
        K = st._kpose()[st.pose_id]
        parked = np.flatnonzero(lat0 & (K < 0))
        applied.append({'letter': [c, i], 'parked': int(len(parked)), 'bprime_on': int((K[off] >= 0).sum()),
                        'parked_equals_lattice_inside_cut': bool(np.array_equal(parked, lat_in)),
                        'lattice_inside_cut': int(len(lat_in)), 'off_after': int((K < 0).sum()),
                        'cap_chambers': int(len(data.cap_members(c)))})
        R.undo()
    assert st.digest() == d0
    ref = json.loads((S / 'v26/B3/singleP.json').read_text())
    ref_set = sorted((r['grip'], r['item']) for r in ref)
    claim = sorted([(237, i) for i in [13, 17, 19] + list(range(34, 43))] +
                   [(257, i) for i in [13, 17, 19] + list(range(34, 43))])
    res = {'Bprime': off.tolist(), 'grips_any_in': grips_any_in, 'grips_all_in': grips_all_in,
           'grips_straddled_none_in': grips_strad_only,
           'grips_any_in_admissible': [c for c in grips_any_in if adm[c]],
           'grips_straddled_total': [c for c in range(600) if not adm[c]],
           'letters_screened': len(rows),
           'letters_em_in_K_any': [[r['grip'], r['item'], r['admissible'], r['n_in']] for r in rows if r['em_in_K']],
           'hits': [list(h) for h in hits], 'n_hits': len(hits),
           'hits_equal_claim': sorted(hits) == claim, 'hits_equal_singleP_json': sorted(hits) == ref_set,
           'applied': applied, 'digest_restored': st.digest() == d0, 'sec': round(time.time() - T0, 1)}
    out.write_text(json.dumps(res) + '\n')
    print(json.dumps({k: v for k, v in res.items() if k not in ('applied',)})[:3000])
    print('parked set', sorted({a['parked'] for a in applied}), 'all eq cut',
          all(a['parked_equals_lattice_inside_cut'] for a in applied), 'bon', sorted({a['bprime_on'] for a in applied}))


if __name__ == '__main__':
    main()
