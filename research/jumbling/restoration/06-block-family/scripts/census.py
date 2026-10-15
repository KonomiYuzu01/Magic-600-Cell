"""Block census for the 3E block-shape family (menu I_a, depth-one off pieces).

For every block of a blocks.py file (pieces with equal Mov), reads the exact poses from the state's pose table and
records: size, Mov, the distinct exact poses, and for each pose its frame K_k^-1 gamma and the positions s = r(p) of
its pieces (e3lib.Lib.canon). A block with one pose has region K_k^-1 gamma A_S; two such blocks lie in one K+-orbit
exactly when their position sets lie in one S'-orbit, so the shape key is the least sorted image of S under S'
(360 elements). Blocks with several poses get a key per pose and are flagged.
usage: census.py <research/jumbling> <classes json> <arrays npz> <out json> NAME=<table json>:<blocks json> ..."""
import hashlib
import json
import sys
import time
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import numpy as np
from e3lib import Lib

J, CL, AR, OUT = sys.argv[1:5]
L = Lib(J, CL, AR)
from sim.kplus import matrix_from_json  # noqa: E402
sig = L.Spairs[:, 1]
t0 = time.time()


def shape_key(S):
    S = np.asarray(sorted(S), np.int64)
    o = L.orb[S]
    best = None
    for g in sig.tolist():
        img = np.sort(L.reg.piece_of[o, L.kp.compose_vec(np.full(len(S), g), L.tr[S])])
        t = tuple(img.tolist())
        if best is None or t < best:
            best = t
    return best


out = {'states': {}}
allkeys = Counter()
for arg in sys.argv[5:]:
    name, rest = arg.split('=', 1)
    tpath, bpath = rest.split(':')
    tab = json.load(open(tpath))
    pid_of, plist = tab['pose_id'], tab['poses']
    blk = json.load(open(bpath))
    assert blk['table_digest'] == tab['digest'], (name, 'blocks and table differ')
    rows = []
    for b in blk['blocks']:
        ps = b['pieces']
        bypose = {}
        for p in ps:
            bypose.setdefault(int(pid_of[p]), []).append(p)
        parts = []
        for key, pl in sorted(bypose.items()):
            Xf = L.fm(matrix_from_json(plist[key]))
            c = L.canon(pl[0], Xf)
            if c is None:
                parts.append({'pieces': len(pl), 'depth_one': False})
                continue
            k, r, _ = c
            S = L.reg.piece_of[L.orb[np.asarray(pl)], L.kp.compose_vec(np.full(len(pl), r), L.tr[np.asarray(pl)])]
            sk = shape_key(S.tolist())
            h = hashlib.sha256(json.dumps(sk).encode()).hexdigest()[:16]
            cls = Counter(int(L.cls_of[s]) for s in S.tolist())
            parts.append({'pieces': len(pl), 'depth_one': True, 'frame_k': int(k), 'shape': h,
                          'classes': len(cls), 'model_orbits': dict(Counter(int(L.orb[p]) for p in pl))})
        keys = sorted(x.get('shape', 'deep') for x in parts)
        bk = hashlib.sha256(json.dumps(keys).encode()).hexdigest()[:16] if len(parts) > 1 else keys[0]
        allkeys[bk] += 1
        rows.append({'size': b['size'], 'movers': b['movers'], 'glued': b['glued'], 'frozen': b['frozen'],
                     'poses': len(parts), 'shape': bk, 'parts': parts, 'first_piece': min(ps)})
    out['states'][name] = {'table_digest': blk['table_digest'], 'off': blk['off'], 'blocks': rows}
    print(name, 'blocks', len(rows), 'poses per block', sorted(Counter(r['poses'] for r in rows).items()),
          '%.0fs' % (time.time() - t0), flush=True)
out['shape_multiplicity'] = dict(allkeys.most_common())
out['distinct_shapes'] = len(allkeys)
json.dump(out, open(OUT, 'w'), indent=1)
print('distinct shapes', len(allkeys), 'repeated', sum(1 for v in allkeys.values() if v > 1))
