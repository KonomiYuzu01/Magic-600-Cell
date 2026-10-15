"""Extra structure check: each conjugating letter x_k is a SAFE-ALL letter at Y_k = E2 . P(k) (k = 0..3), with P(k) rebuilt
from R = (237,13) and x_0..x_3 by the own mirror inverse; each Y_k digest is compared with the shrink record.

Usage: python3 -I -B xk_check.py <out json>
"""
import json
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
SP = Path('<work>')
TH = SP / 'wt-e3' / 'research' / 'jumbling' / 'theory'
B4 = SP / 'v26' / 'B4'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))

import blocks as BL  # noqa: E402
import buffer as B  # noqa: E402
from exact import identity, matmul  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

O, restore = BL.O, BL.restore
I4 = to_tuple(identity(4))
T0 = time.time()


def main():
    R, M = O.start(str(B4 / 'E2-table.json'))
    st = R.st
    d0 = st.digest()

    def minv(w):
        c, i = w
        hits = [j for j in range(len(M.items))
                if to_tuple(matmul(restore.element(M, c, i), restore.element(M, c, j))) == I4]
        assert len(hits) == 1
        return (c, hits[0])
    xs = [(91, 6), (154, 7), (133, 1), (183, 6)]
    levels = [[(237, 13)]]
    for x in xs:
        levels.append(levels[-1] + [x] + [minv(w) for w in reversed(levels[-1])])
    shrink = [json.loads(x) for x in (B4 / 'shrink-237-13.jsonl').read_text().splitlines() if x.strip()]
    rows = []
    for k in range(4):
        for w in levels[k]:
            R.apply(*w)
        dk = st.digest()
        G = B.Alphabet(R, M)
        x = xs[k]
        rows.append({'k': k, 'P_length': len(levels[k]), 'digest': dk, 'digest_equals_shrink': dk == shrink[k]['after_P_digest'],
                     'off': int(len(G.holes)), 'safe_grips': len(G.safe), 'letters': len(G.letters),
                     'x_k': list(x), 'x_k_safe_all_letter': x in G.index,
                     'x_k_grip_status': G.blocked.get(x[0], 'safe'), 'sec': round(time.time() - T0, 1)})
        print(json.dumps(rows[-1]), file=sys.stderr, flush=True)
        del G
        for _ in levels[k]:
            R.undo()
        assert st.digest() == d0
    ok = all(r['digest_equals_shrink'] and r['x_k_safe_all_letter'] for r in rows)
    Path(sys.argv[1]).write_text(json.dumps({'ok': ok, 'rows': rows, 'seconds': round(time.time() - T0, 1)}, indent=1) + '\n')
    print(json.dumps({'ok': ok}))


if __name__ == '__main__':
    main()
