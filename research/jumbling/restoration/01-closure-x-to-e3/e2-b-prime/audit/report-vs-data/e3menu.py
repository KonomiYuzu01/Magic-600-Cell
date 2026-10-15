"""report-vs-data audit: SAFE-ALL menu, orbits and holes at E3.

Usage: python3 -I e3menu.py <theory dir> <E3 table> <out json>
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import blocks as BL  # noqa: E402
import buffer as B  # noqa: E402


def main():
    a = sys.argv[1:]
    R, M = BL.O.start(a[1])
    st = R.st
    G = B.Alphabet(R, M)
    orb, size = G.orbits()
    K = st._kpose()[st.pose_id]
    out = {'digest': st.digest(), 'off': int((K < 0).sum()), 'holes': len(G.holes), 'safe': len(G.safe),
           'letters': len(G.letters), 'orbits': int(len(size)), 'singletons': int((size == 1).sum()),
           'blocked': sorted(int(c) for c in G.blocked)}
    Path(a[2]).write_text(json.dumps(out) + '\n')
    print(json.dumps(out))


if __name__ == '__main__':
    main()
