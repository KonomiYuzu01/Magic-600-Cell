"""J3-R diagnosis: visible quantities along the true scramble path of a W-J fixture (restoration.md, section 5).

For every prefix t of the fixture's records, the configuration X_t after t records gives
  N        the total misalignment (rim.Domains.total_misalignment),
  H        the total tree height (sum over pieces of d_L + d_R, trees.pose_tree),
  off      the number of pieces off the lattice (pose not in K+),
  domains  the number of domains (cosets gamma K+ present).
Read backwards, the records are a restoring word, so for each quantity q and each t the script
reports the length of the shortest prefix of that reverse path that lowers q: the smallest k with
q(X_{t-k}) < q(X_t). This uses the journal and is a diagnosis of the method, never a rule of it.

Run from the repository root (writes results/paths-<menu>-<t>.json):
    python research/jumbling/theory/paths.py <menu> <t>
Evidence kind: source and exact synthetic geometry.
"""
import collections
import json
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))

import rim  # noqa: E402
import sim  # noqa: E402
import trees  # noqa: E402
import wj  # noqa: E402

ctx = sim.get_context()


def series(name, T):
    fixture, M = wj.load(ctx, name)
    st = sim.State(ctx, menu=M)
    heights = {}
    Q = {'N': [0], 'H': [0], 'off': [0], 'domains': [1]}
    for rec in fixture['journal']['records'][:T]:
        if not st.apply(sim.Twist.from_record(ctx, rec)).applied:
            raise RuntimeError('fixture record not admissible')
        D = rim.Domains(st)
        Q['N'].append(D.total_misalignment())
        Q['domains'].append(len(D.reps))
        count = np.bincount(st.pose_id, minlength=len(st._poses))
        H = off = 0
        for pid in np.nonzero(count)[0].tolist():
            p = st._poses[pid]
            if p.kidx >= 0:
                continue
            off += int(count[pid])
            if p.key not in heights:
                dl, dr, _, _ = trees.pose_tree(st._pose_matrix(pid))
                heights[p.key] = dl + dr
            H += int(count[pid]) * heights[p.key]
        Q['H'].append(H)
        Q['off'].append(off)
    return Q


def reverse_lowering(v):
    """For each t >= 1, the smallest k with v[t-k] < v[t] (None if no prefix lowers v)."""
    return [next((k for k in range(1, t + 1) if v[t - k] < v[t]), None) for t in range(1, len(v))]


def main(name, T):
    Q = series(name, T)
    out = {'fixture': name, 'records': T, 'series': Q, 'reverse_lowering': {}}
    for q, v in Q.items():
        need = reverse_lowering(v)
        ok = [k for k in need if k is not None]
        out['reverse_lowering'][q] = {
            'per_t': need, 'max': max(ok) if ok else None, 'mean': round(sum(ok) / len(ok), 2) if ok else None,
            'histogram': {str(k): n for k, n in sorted(collections.Counter(ok).items())}, 'never': need.count(None)}
    path = HERE / 'results' / f'paths-{name}-{T}.json'
    path.write_text(json.dumps(out) + '\n')
    print(name, T, json.dumps({q: {k: s[k] for k in ('max', 'mean', 'never')} for q, s in out['reverse_lowering'].items()}))


if __name__ == '__main__':
    if len(sys.argv) != 3:
        print(__doc__)
    else:
        main(sys.argv[1], int(sys.argv[2]))
