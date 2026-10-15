"""Resource table of a state, read from its pose table only: holes, off blocks and their blocking, SAFE-ALL menu.

For each rigid off block: size, smallest piece, distinct exact poses, grips whose cap holds it with an admissible cut
(inside grips), grips whose cut it straddles, and its locked chambers with their occupants (blocks.Blocks). For the
state: holes, straddled grips, the SAFE-ALL menu rebuilt here (buffer.Alphabet: grips, letters, blocked grips and why,
orbits) and, with a reference state, the SAFE-ALL grips gained and lost against it.

Usage: python -I resources.py <theory dir> <pose table> <out json> [<reference pose table>]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import blocks as BL  # noqa: E402
import buffer as B  # noqa: E402
from sim.kplus import matrix_json  # noqa: E402

CL, O = BL.CL, BL.O
T0 = time.time()


def read(table):
    R, M = O.start(table)
    st = R.st
    bl = BL.Blocks(R, M)
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    K = st._kpose()[st.pose_id]
    lat = np.flatnonzero(K >= 0)
    occ = np.full(len(K), -1, np.int64)
    occ[reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]] = lat
    blocks = []
    for b, pcs in enumerate(bl.blocks):
        poses = sorted({json.dumps(matrix_json(st.pose(int(p)))) for p in pcs})
        blocks.append({'block': b, 'size': len(pcs), 'smallest_piece': int(pcs[0]), 'pieces': pcs.tolist(),
                       'distinct_poses': len(poses), 'poses': [json.loads(x) for x in poses],
                       'inside_grips': bl.I[b].tolist(), 'straddled_grips': bl.S[b].tolist(),
                       'locked_chambers': bl.locked[b], 'locked_contents': [int(occ[x]) for x in bl.locked[b]],
                       'vacant_locked_chambers': bl.vacant_locked[b]})
    G = B.Alphabet(R, M)
    G.orbits()
    return {'digest': st.digest(), 'off': int((K < 0).sum()), 'holes': np.flatnonzero(occ < 0).tolist(),
            'blocks': blocks, 'straddling_grips': bl.straddling_grips.tolist(),
            'safe_all': {'grips': list(map(int, G.safe)), 'grip_count': len(G.safe), 'letters': len(G.letters),
                         'blocked': {str(k): v for k, v in G.blocked.items()}, 'orbits': int(len(G.orbit_size)),
                         'singleton_orbits': int((G.orbit_size == 1).sum()),
                         'largest_orbits': sorted(map(int, G.orbit_size))[-8:]}}


def main():
    a = sys.argv[1:]
    res = read(a[1])
    if len(a) > 3:
        ref = read(a[3])
        res['reference'] = {'digest': ref['digest'], 'off': ref['off'], 'holes': len(ref['holes']),
                            'safe_grip_count': ref['safe_all']['grip_count'], 'letters': ref['safe_all']['letters'],
                            'orbits': ref['safe_all']['orbits']}
        res['safe_gained'] = sorted(set(res['safe_all']['grips']) - set(ref['safe_all']['grips']))
        res['safe_lost'] = sorted(set(ref['safe_all']['grips']) - set(res['safe_all']['grips']))
    res['seconds'] = round(time.time() - T0, 1)
    Path(a[2]).write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps({'digest': res['digest'][:16], 'off': res['off'], 'holes': len(res['holes']),
                      'blocks': [(b['size'], b['inside_grips'], b['straddled_grips'], b['locked_chambers'],
                                  b['locked_contents']) for b in res['blocks']],
                      'safe': res['safe_all']['grip_count'], 'letters': res['safe_all']['letters'],
                      'orbits': res['safe_all']['orbits'], 'gained': res.get('safe_gained'),
                      'lost': res.get('safe_lost')}))


if __name__ == '__main__':
    main()
