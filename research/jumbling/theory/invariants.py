"""J3 items 3 and 4: exact checks behind Proposition 4.1 (centre points are fixed) and the
infinite-order certificate of the E2 witness. Exact in Q(sqrt5) throughout.

    python research/jumbling/theory/invariants.py <out.json> [--replay]

1. For every cap c, the centre piece (signature {c}) contains p_c = alpha n_c in its closed home
   region: n_c . p_c = kappa, n_e . p_c <= kappa and n_e . p_c <= |n|^2 for every pole e.
2. The E2 witness g (plane rotation fixing n_0 and n_13 by about 10 degrees) has a trace whose
   field trace to Q is not an integer, so tr g is not an algebraic integer and g has infinite order.
3. With --replay: every W-J fixture journal is replayed by J1, and after every applied twist each
   centre piece's pose fixes its own pole exactly (g_c n_c = n_c). Evidence for Proposition 4.1,
   not part of its proof.
"""
import json
import sys
import time
from fractions import Fraction
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))
import numpy as np  # noqa: E402
import sim  # noqa: E402
from exact import Q5, ZERO, dot, matvec  # noqa: E402

ctx = sim.get_context()
data = ctx.data
N, NN, KAPPA, ALPHA = data.N, data.NN, data.KAPPA, data.ALPHA


def q5_text(x):
    return f'({x.a} + {x.b} sqrt5) / {x.d}'


def centre_pieces():
    centres = []
    for c in range(600):
        found = [int(p) for p in data.cap_members(c) if data.signature(int(p)) == frozenset([c])]
        assert len(found) == 1, f'cap {c}: {len(found)} pieces with signature {{{c}}}'
        centres.append(found[0])
    return centres


def centre_points():
    """p_c = alpha n_c satisfies every inequality of the centre's closed region (signature {c})."""
    failures = 0
    for c in range(600):
        p = [ALPHA * x for x in N[c]]
        if dot(N[c], p) != KAPPA:
            failures += 1
            continue
        for e in range(600):
            v = dot(N[e], p)
            if (v - NN).sign() > 0 or (e != c and (v - KAPPA).sign() > 0):
                failures += 1
                break
    return failures


def e2_trace():
    g = sim.plane(ctx, 0, 13, degrees=10).matrix
    tr = sum((g[k][k] for k in range(4)), ZERO)
    field_trace = Fraction(2 * tr.a, tr.d)            # Tr_{Q(sqrt5)/Q}((a + b sqrt5)/d) = 2a/d
    norm = Fraction(tr.a * tr.a - 5 * tr.b * tr.b, tr.d * tr.d)
    return {'trace_q_sqrt5': q5_text(tr), 'field_trace': str(field_trace), 'field_norm': str(norm),
            'algebraic_integer': field_trace.denominator == 1 and norm.denominator == 1,
            'fixes_poles_0_and_13': matvec(g, N[0]) == list(N[0]) and matvec(g, N[13]) == list(N[13])}


def replay(centres):
    import wj
    out = {}
    home = {piece: c for c, piece in enumerate(centres)}
    for name in wj.MENUS:
        fixture, menu = wj.load(ctx, name)
        st = sim.State(ctx, menu=menu)
        ok_pose = {0}                                 # pose ids known to fix the pole of their centre
        checked, rotated_max, t0 = 0, 0, time.time()
        cpieces = np.array(centres, np.int64)
        for i, record in enumerate(fixture['journal']['records']):
            out_i = st.apply(sim.Twist.from_record(ctx, record))
            assert out_i.applied, f'{name} record {i}: {out_i.status}'
            for piece in cpieces[st.pose_id[cpieces] != 0]:
                pid = int(st.pose_id[piece])
                c = home[int(piece)]
                if (pid, c) in ok_pose:
                    continue
                m = st.pose(int(piece))
                assert matvec(m, N[c]) == list(N[c]), f'{name} record {i}: centre {c} left its pole'
                ok_pose.add((pid, c))
                checked += 1
            rotated_max = max(rotated_max, int((st.pose_id[cpieces] != 0).sum()))
        out[name] = {'records': len(fixture['journal']['records']), 'distinct_centre_poses_checked': checked,
                     'centres_rotated_in_place_at_end': int((st.pose_id[cpieces] != 0).sum()),
                     'most_centres_rotated_at_once': rotated_max, 'seconds': round(time.time() - t0, 1)}
        print(name, out[name], flush=True)
    return out


if __name__ == '__main__':
    centres = centre_pieces()
    res = {'centre_pieces': len(set(centres)), 'centre_point_failures': centre_points(), 'e2': e2_trace()}
    print({k: v for k, v in res.items()}, flush=True)
    assert res['centre_pieces'] == 600 and res['centre_point_failures'] == 0
    assert not res['e2']['algebraic_integer'] and res['e2']['fixes_poles_0_and_13']
    if '--replay' in sys.argv:
        res['replay'] = replay(centres)
    Path(sys.argv[1]).write_text(json.dumps(res, indent=1) + '\n')
