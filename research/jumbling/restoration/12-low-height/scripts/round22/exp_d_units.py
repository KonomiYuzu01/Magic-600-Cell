# Round 22, line (c), experiment D (R22C2-02): the units of a pose table as in exp_b.py (blocks split by domain),
# written as piece lists; with unit files, the index of the unit whose pieces equal each file's. Exact; headless.
# Usage (repository root): python3 -I -B exp_d_units.py <table> <out dir> <n>...   or   ... <table> --find <unit json>...
import json
import sys
from pathlib import Path

import numpy as np

REPO = Path.cwd()
J = REPO / 'research/jumbling'
sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures')]
import buffer as B  # noqa: E402
import blocks as BL  # noqa: E402
import rim  # noqa: E402

O = B.O


def units_of(table):
    R, M = O.start(table)
    st = R.st
    bl = BL.Blocks(R, M)
    dom = np.asarray(rim.Domains(st).dom)
    out = []
    for pieces in bl.blocks:
        pieces = np.asarray(pieces)
        for d in np.unique(dom[pieces]).tolist():
            out.append((sorted(int(p) for p in pieces[dom[pieces] == d]),
                        sorted(int(p) for p in pieces[dom[pieces] != d])))
    return st.digest(), out


def main():
    table = sys.argv[1]
    z, units = units_of(table)
    if sys.argv[2] == '--find':
        for f in sys.argv[3:]:
            want = sorted(json.loads(Path(f).read_text())['pieces'])
            hit = [i for i, (T, F) in enumerate(units) if T == want]
            print(json.dumps({'unit_file': Path(f).name, 'digest': z, 'index': hit, 'units': len(units)}), flush=True)
        return
    od = Path(sys.argv[2])
    od.mkdir(parents=True, exist_ok=True)
    for n in map(int, sys.argv[3:]):
        T, F = units[n]
        assert not F, f'unit {n} has other frames in its block'
        (od / f'u{n}.json').write_text(json.dumps({'state_digest': z, 'unit': n, 'size': len(T), 'pieces': T}) + '\n')
        print(json.dumps({'unit': n, 'size': len(T), 'first': T[0], 'digest': z}), flush=True)


if __name__ == '__main__':
    main()
