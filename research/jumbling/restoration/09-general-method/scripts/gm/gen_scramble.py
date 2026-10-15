"""Fresh random I_a scrambles for the general-driver tests: k admissible jumble twists at uniformly random grips
from the solved state (theory/restore.py scramble, menu I_a), exported as current-only pose tables.
Research only (headless, exact synthetic geometry).

usage: gen_scramble.py <theory dir> <out dir> <k>:<seed> ...
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import buffer as B  # noqa: E402
import restore  # noqa: E402

out = Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)
M = restore.menu('I_a')
for spec in sys.argv[3:]:
    k, seed = (int(x) for x in spec.split(':'))
    st, hist = restore.scramble(M, k, seed)
    name = f'ia-k{k}-s{seed}'
    B.export_pose_table(st, str(out / f'{name}-table.json'))
    print(json.dumps({'name': name, 'twists': k, 'seed': seed, 'off': int(st.off_lattice_count()),
                      'digest': st.digest()}), flush=True)
