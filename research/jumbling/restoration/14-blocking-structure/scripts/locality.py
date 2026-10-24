"""Exact L1 locality graph. Run from the repository root; output must be external.

The delta matrix is losslessly encoded by delta_values[delta_index[c][d]].
No angle or floating point approximation participates in a decision.
"""
import argparse
from functools import lru_cache
import json
import os
from pathlib import Path
import sys
import time

sys.dont_write_bytecode = True
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_name] = '1'
sys.path.insert(0, str(Path(__file__).resolve().parent))
import blockmap as BM  # noqa: E402
from exact import dot  # noqa: E402


def exact_string(q):
    if not q.b:
        s = str(q.a)
    elif not q.a:
        s = f'{q.b}*sqrt(5)'
    else:
        s = f'{q.a} {"+" if q.b > 0 else "-"} {abs(q.b)}*sqrt(5)'
    return s if q.d == 1 else f'({s})/{q.d}'


def external_path(path):
    p = Path(path).resolve()
    if p.is_relative_to(Path(__file__).resolve().parents[5]):
        raise ValueError('outputs and caches must be outside the repository')
    return p


def write_json(path, data):
    p = external_path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(data, sort_keys=True, indent=1, allow_nan=False) + '\n')


@lru_cache(maxsize=1)
def graph():
    """All 600 x 600 deltas, including the unused diagonal, in Q(sqrt(5))."""
    C = BM.C
    r2 = max(dot(v, v) for v in C.regions.cap_vertices)
    poles, nn, kappa = C.data.N, C.data.NN, C.data.KAPPA
    deltas = [[None] * 600 for _ in range(600)]
    partners = [[] for _ in range(600)]
    for c in range(600):
        for d in range(c, 600):
            delta = r2 * (nn + dot(poles[c], poles[d])) - 2 * kappa * kappa
            deltas[c][d] = deltas[d][c] = delta
            if c != d and delta.sign() > 0:
                partners[c].append(d)
                partners[d].append(c)
    return r2, deltas, partners


def analyse():
    start = time.process_time()
    r2, deltas, partners = graph()
    palette = sorted(set(deltas[0]))
    index = {v: i for i, v in enumerate(palette)}
    shells = {}
    for d in range(600):
        v = dot(BM.C.data.N[0], BM.C.data.N[d])
        shells.setdefault(v, []).append(d)
    counts = [len(row) for row in partners]
    return dict(
        format='t3p-locality/1', R2=exact_string(r2),
        NN=exact_string(BM.C.data.NN), kappa=exact_string(BM.C.data.KAPPA),
        delta_encoding='delta(c,d) = delta_values[delta_index[c][d]]; diagonal is not a partner',
        delta_values=[exact_string(v) for v in palette],
        delta_index=[[index[v] for v in row] for row in deltas],
        partners=partners, partner_counts=counts,
        equal_partner_counts=len(set(counts)) == 1,
        partner_count=counts[0] if len(set(counts)) == 1 else None,
        shells=[dict(dot=exact_string(v), count=len(ds), grips=ds,
                     delta=exact_string(deltas[0][ds[0]]),
                     interacts=bool(ds[0] != 0 and deltas[0][ds[0]].sign() > 0))
                for v, ds in sorted(shells.items(), reverse=True)],
        cpu_s=round(time.process_time() - start, 6))


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('out')
    args = ap.parse_args(argv)
    external_path(args.out)
    write_json(args.out, analyse())


if __name__ == '__main__':
    main()
