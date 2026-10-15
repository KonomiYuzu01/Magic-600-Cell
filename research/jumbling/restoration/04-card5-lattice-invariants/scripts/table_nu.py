"""nu of a lattice configuration given as an exact pose table (poses, pose_id, digest); the table digest is checked.
usage: table_nu.py <research/jumbling> <repo root> <nucoords dir> <table json> ..."""
import hashlib, json, sys
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
sys.path.insert(0, sys.argv[3])
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import restore as RS
import sim
from sim.state import State, Pose
from sim.kplus import matrix_from_json
from nucoords import NuCoords
ctx = RS.ctx
M = RS.menu('I_a')
NC = NuCoords(sys.argv[2])
for path in sys.argv[4:]:
    doc = json.loads(Path(path).read_text())
    st = State(ctx, menu=M)
    st._poses, st._lookup = [Pose(0, None, ('K', 0))], {('K', 0): 0}
    ids = np.array([st._intern_matrix(matrix_from_json(m)) for m in doc['poses']], np.int32)
    st.pose_id = ids[np.asarray(doc['pose_id'], np.int64)]
    st._compact()
    st.journal, st._anchor_cache = [], {}
    ok = st.digest() == doc['digest']
    lat = bool(st.is_lattice())
    nu = {str(k): [int(x[0]), int(x[1])] for k, x in sorted(NC.nu(np.asarray(st.lattice_stickers()['labels'], np.int64)).items())} if lat else None
    print(json.dumps({'table': path, 'sha256': hashlib.sha256(Path(path).read_bytes()).hexdigest(), 'digest_ok': ok, 'lattice': lat, 'nu': nu}), flush=True)
