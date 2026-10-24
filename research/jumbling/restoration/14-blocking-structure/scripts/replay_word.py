# Stage 14 (blocking structure), tool 6: replay a literal word from a stored state, independently of the search tools.
#
# Each letter (grip, menu item) is applied with the exact simulator; after each letter every left height is recomputed
# from scratch and the row records whether the letter was admissible, n2, n4, n6, the depth Phi = sum of
# max(0, h - 2), the largest height, the off count and the state digest. The replay stops at the first letter that is
# not admissible. Exact geometry; research only, headless Linux; no Windows, DirectX, input or performance claim.
#
# Run from the repository root:
#   python3 -I -B research/jumbling/restoration/14-blocking-structure/scripts/replay_word.py <table.json>
#       <c:i,c:i,...> <out.json>
import argparse
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import blockmap as BM  # noqa: E402

C, M, R, sim = BM.C, BM.M, BM.R, BM.sim


def summary(st):
    h, _ = BM.tree_info(st)
    h = h.astype(np.int64)
    return dict(n2=int((h == 2).sum()), n4=int((h == 4).sum()), n6=int((h == 6).sum()), max_h=int(h.max()),
                phi=int(np.maximum(h - 2, 0).sum()), off=int(st.off_lattice_count()), digest=st.digest())


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('table')
    ap.add_argument('word', help='letters c:i separated by commas, i a menu item 0 to 59')
    ap.add_argument('out')
    a = ap.parse_args(argv)
    st = BM.load_table(a.table)
    word = [tuple(int(v) for v in t.split(':')) for t in a.word.split(',')]
    rows = [dict(letter=None, applied=None, **summary(st))]
    print(json.dumps(rows[-1]), flush=True)
    for c, i in word:
        o = st.apply(sim.Twist(C, c, R.element(M, c, i), 'menu', {'menu': M.name}), journal=False)
        assert not o.uncertain
        if not o.applied:
            rows.append(dict(letter=[c, i], applied=False))
            print(json.dumps(rows[-1]), flush=True)
            break
        rows.append(dict(letter=[c, i], applied=True, **summary(st)))
        print(json.dumps(rows[-1]), flush=True)
    Path(a.out).write_text(json.dumps(dict(table=a.table, word=[list(w) for w in word], rows=rows), indent=1))
    return 0 if all(r['applied'] is not False for r in rows) else 1


if __name__ == '__main__':
    sys.exit(main())
