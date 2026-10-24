# Stage 14 (blocking structure), tool 4: co-held and dominating foreign pieces of the high units (Lemma C).
#
# For each high unit G (a pose group of height 4 or more) of a state, from its label table (blockmap.py --labels):
# - its holders (the grips whose caps hold every piece of G whole) and its own holders (those of its branch's class);
# - the co-held foreign pieces: pieces of another branch at height 2 or more whose holder set equals the holder set of
#   every member of G (Lemma C is per member, so when the members' holder sets differ no piece qualifies). Every twist
#   moves them exactly when it moves G, retained twists keep the holder sets equal (Corollary L1), so a lowering of G
#   after retained twists only also raises them (Lemma C). The count against G's common holder set (the intersection
#   over its members), which is not enough for Lemma C, is reported separately;
# - the dominating foreign pieces: pieces of another branch at height 2 or more whose holder set contains G's. Each of
#   them lies in the foreign load of every own holder of G.
# Exact geometry (through the label table); research only, headless Linux; no Windows, DirectX, input or performance
# claim.
#
# Run from the repository root:
#   python3 -I -B research/jumbling/restoration/14-blocking-structure/scripts/coheld.py <table.json> <labels.npz>
#       <out.json>
import argparse
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import blockmap as BM  # noqa: E402

CLS = BM.CLS


def coheld(st, lab, h, b):
    inside = lab == 1
    cand = np.flatnonzero(h >= 2)
    hold = inside[cand]
    out = []
    for g in np.unique(st.pose_id[h >= 4]).tolist():
        mem = np.flatnonzero(st.pose_id == g)
        rows = inside[mem]
        hg = rows.all(0)
        uniform = bool((rows == rows[0]).all())
        foreign = b[cand] != b[mem[0]]
        equal = (hold == hg).all(1)
        contains = (hold | ~hg).all(1)
        common, dom = cand[equal & foreign], cand[contains & foreign]
        co = common if uniform else common[:0]
        out.append(dict(pose=int(g), size=int(len(mem)), height=int(h[mem[0]]), branch=int(b[mem[0]]),
                        holders=np.flatnonzero(hg).tolist(), own=np.flatnonzero(hg & (CLS == b[mem[0]])).tolist(),
                        member_holder_rows=int(len(np.unique(rows, axis=0))),
                        coheld_foreign=int(len(co)), coheld_foreign_heights=sorted(set(h[co].tolist())),
                        common_holder_set_foreign=int(len(common)),
                        dominating_foreign=int(len(dom)), dominating_foreign_heights=sorted(set(h[dom].tolist()))))
    return out


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('table')
    ap.add_argument('labels')
    ap.add_argument('out')
    a = ap.parse_args(argv)
    st = BM.load_table(a.table)
    h, b = BM.tree_info(st)
    lab = BM.labels_for(st, a.labels, 1)
    rows = coheld(st, lab, h, b)
    Path(a.out).write_text(json.dumps(dict(digest=st.digest(), groups=rows), indent=1))
    for r in rows:
        print(json.dumps({k: r[k] for k in ('pose', 'size', 'branch', 'own', 'member_holder_rows', 'coheld_foreign',
                                            'common_holder_set_foreign', 'dominating_foreign')}), flush=True)


if __name__ == '__main__':
    main()
