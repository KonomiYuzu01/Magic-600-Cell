# E-B6: which caps can meet. In a jumbled state the pieces do not tile the body (the regions of off pieces stick out of
# it; stage 12 README, section 4), but every piece lies in the closed ball of radius R about the centre, with R^2 the
# largest |v|^2 over the vertices of the home cells (twists are rotations, and a home cell lies in the hull of its
# vertices). The caps of x and y, read inside that ball, share a point exactly when R cos(phi/2) >= alpha |n|, with phi
# the angle between the poles; in exact form (NN + <n_x, n_y>) R^2 >= 2 KAPPA^2 with NN = |n|^2 and KAPPA = alpha NN.
# eb6.py looks for the second cap of a commutator among the caps within 55.7 degrees of the first (restore.NEAR); this
# checks, in exact arithmetic, that the list holds every cap that can meet it, and records at which angles caps can
# meet.
# Exact (Q(sqrt 5)); research only, headless Linux.
# Run from the repository root:
#   python3 -I -B eb6_meet.py <out json>
import json
import sys
from pathlib import Path

import numpy as np

REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures')]
import restore as R  # noqa: E402
from exact import dot  # noqa: E402


def main():
    D, reg = R.ctx.data, R.ctx.regions
    R2 = max((dot(v, v) for verts in reg.rep_vertices for v in verts), key=float)
    assert all((dot(v, v) - R2).sign() <= 0 for verts in reg.rep_vertices for v in verts)
    bound = D.KAPPA * D.KAPPA * 2
    values = {}                                          # exact <n_x, n_y> -> ordered pairs x != y
    for x in range(600):
        for y in range(600):
            if x != y:
                values.setdefault(dot(D.N[x], D.N[y]), []).append((x, y))
    meet = np.zeros((600, 600), bool)
    rows = []
    for g, pairs in values.items():
        m = ((D.NN + g) * R2 - bound).sign() >= 0
        xs, ys = zip(*pairs)
        meet[list(xs), list(ys)] = m
        ang = float(np.degrees(np.arccos(float(g) / float(D.NN))))
        rows.append({'angle_deg': round(ang, 2), 'inner_product': [g.a, g.b, g.d], 'ordered_pairs': len(pairs),
                     'caps_can_meet': bool(m)})
    rows.sort(key=lambda r: r['angle_deg'])
    near = np.zeros((600, 600), bool)
    for x in range(600):
        near[x, R.NEAR[x]] = True
    np.fill_diagonal(near, False)
    met = [r['angle_deg'] for r in rows if r['caps_can_meet']]
    res = {'R_squared': [R2.a, R2.b, R2.d], 'R': round(float(R2) ** 0.5, 6),
           'cut_alpha_n': round(float(D.KAPPA) / float(D.NN) ** 0.5, 6),
           'two_theta_deg': round(2 * float(np.degrees(np.arccos(float(D.KAPPA) / (float(D.NN) * float(R2)) ** 0.5))),
                                  3),
           'meeting_ordered_pairs': int(meet.sum()), 'meeting_per_cap': sorted(set(meet.sum(axis=1).tolist())),
           'largest_meeting_angle_deg': max(met), 'smallest_non_meeting_angle_deg': min(
               r['angle_deg'] for r in rows if not r['caps_can_meet']),
           'near_ordered_pairs': int(near.sum()), 'near_per_cap': sorted(set(near.sum(axis=1).tolist())),
           'meeting_outside_near': int((meet & ~near).sum()),
           'angles': [r for r in rows if r['angle_deg'] < 60]}
    Path(sys.argv[1]).write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps({k: v for k, v in res.items() if k != 'angles'}))
    assert res['meeting_outside_near'] == 0


if __name__ == '__main__':
    main()
