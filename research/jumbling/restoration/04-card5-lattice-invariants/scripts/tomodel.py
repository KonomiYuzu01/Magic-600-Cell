"""Rewrite harvest rows so that 'hole_orbit' and 'off_orbit' use the model orbit_id (assets/model.npz), as the
Card 5 tools expect; the registry-index values are kept under '*_registry_index'.
usage: tomodel.py <research/jumbling> <in jsonl> ... > out.jsonl"""
import json, sys
from pathlib import Path
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import numpy as np
import restore as RS
oid = np.asarray(RS.ctx.data.orbit_id)
for path in sys.argv[2:]:
    for line in open(path):
        if not line.strip():
            continue
        r = json.loads(line)
        r['hole_orbit_registry_index'] = r.get('hole_orbit')
        r['off_orbit_registry_index'] = r.get('off_orbit')
        r['hole_orbit'] = [int(oid[h]) for h in r['holes']]
        r['off_orbit'] = [int(oid[p]) for p in r['off']]
        print(json.dumps(r))
