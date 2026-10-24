"""Greatest-fixed-point unlock levels and per-member high-unit holder rows."""
import argparse
from collections import Counter
import os
from pathlib import Path
import sys
import time

sys.dont_write_bytecode = True
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_name] = '1'
sys.path.insert(0, str(Path(__file__).resolve().parent))
import numpy as np  # noqa: E402
import blockmap as BM  # noqa: E402
from locality import external_path, write_json  # noqa: E402

# A finite level can traverse at most the 600 grips before reaching a clean one.
# Saturation at this sentinel implements infinity, not a finite depth cutoff.
INF = 601


def display(value):
    return 'infinity' if int(value) == INF else int(value)


def fixed_point(lab):
    clean = ~(lab == 2).any(axis=0)
    pieces = np.flatnonzero((lab == 2).any(axis=1))
    rows = lab[pieces]
    straddle_piece, straddle_grip = np.nonzero(rows == 2)
    lev = np.where(clean, 0, INF).astype(np.int32)
    iterations = 0
    while True:
        minimum = np.full(len(pieces), INF, np.int32)
        for c in np.flatnonzero(lev < INF).tolist():
            held = rows[:, c] == 1
            minimum[held] = np.minimum(minimum[held], lev[c])
        nxt = np.zeros(600, np.int32)
        np.maximum.at(nxt, straddle_grip, minimum[straddle_piece])
        nxt[~clean] = np.minimum(INF, nxt[~clean] + 1)
        assert (nxt <= lev).all(), 'iteration from infinity must be decreasing'
        iterations += 1
        if np.array_equal(nxt, lev):
            return lev, iterations
        lev = nxt


def analyse(state):
    start = time.process_time()
    lab, h, b, st = state.lab, state.h, state.b, state.st
    lev, iterations = fixed_point(lab)
    units = []
    high_pieces = np.flatnonzero(h >= 4)
    for pid in np.unique(st.pose_id[high_pieces]).tolist():
        members = np.flatnonzero(st.pose_id == pid)
        held = lab[members] == 1
        counts = held.sum(axis=0)
        whole = counts == len(members)
        own = np.flatnonzero(whole & (BM.CLS == b[members[0]])).tolist()
        distinct = len(np.unique(np.packbits(held, axis=1), axis=0))
        units.append(dict(
            pose=pid, unit_piece=int(members[0]), size=len(members),
            height=int(h[members[0]]), branch=int(b[members[0]]),
            own_holders=[dict(grip=c, level=display(lev[c])) for c in own],
            cell_split=dict(scope='holder rows at all 600 grips, separately for every member',
                            distinct_holder_rows=distinct, intersection_size=int(whole.sum()),
                            union_size=int((counts > 0).sum()),
                            clean_partial_holders=[dict(grip=c, members=int(counts[c]))
                                                   for c in np.flatnonzero(
                                                       (lev == 0) & (counts > 0) & ~whole).tolist()])))
    best = None
    for c in range(600):
        ps = high_pieces[lab[high_pieces, c] == 1]
        if len(ps):
            candidate = (int(lev[c]), int(ps[0]), c)
            if best is None or candidate < best:
                best = candidate
    return dict(format='t3p-levels/1', digest=st.digest(),
                levels=[display(v) for v in lev],
                histogram=dict(sorted(Counter(str(display(v)) for v in lev).items())),
                iterations=iterations, high_units=units,
                **{'lambda': display(best[0]) if best else 'infinity',
                   'lambda_witness': dict(piece=best[1], holder=best[2]) if best else None},
                cpu_s=round(time.process_time() - start, 6))


def main(argv=None):
    from t3p import load_input
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('table')
    ap.add_argument('out')
    ap.add_argument('--word', default='')
    ap.add_argument('--labels')
    args = ap.parse_args(argv)
    external_path(args.out)
    state, setup = load_input(args.table, args.word, args.labels)
    result = analyse(state)
    result['setup'] = setup
    write_json(args.out, result)


if __name__ == '__main__':
    main()
