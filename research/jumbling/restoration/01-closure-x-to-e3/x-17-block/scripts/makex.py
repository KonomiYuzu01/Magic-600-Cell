"""Generate the new input X from a seed pose table by a fixed generating word u; write X and its certificate.

u = the given leading letters (checked admissible), then a seeded random SAFE-ALL word of the given length read
at the state after them (SAFE-ALL: cut not straddled, no off piece in the cap, element in K). The generating word,
the digests after every letter and the seed's digest go to the certificate; X's current-only pose table goes to
its own file. The restoration constructor reads only X's table.

Usage: python -I makex.py <theory dir> <seed table> <lead letters json> <safe length> <seed> <cert out> <table out>
"""
import json
import random
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import buffer as B  # noqa: E402

O = B.O
T0 = time.time()
a = sys.argv[1:]
seed_table, lead = a[1], [(int(c), int(i)) for c, i in json.loads(a[2])]
n_safe, seed, cert_out, table_out = int(a[3]), int(a[4]), a[5], a[6]
R, M = O.start(seed_table)
st = R.st
d0 = st.digest()
u, digests = [], []
for c, i in lead:
    R.apply(c, i)
    u.append([c, i])
    digests.append(st.digest())
G = B.Alphabet(R, M)
rng = random.Random(seed)
safe_word = [rng.randrange(len(G.letters)) for _ in range(n_safe)]
for n in safe_word:
    c, i = G.letters[n][0], G.letters[n][1]
    R.apply(c, i)
    u.append([int(c), int(i)])
    digests.append(st.digest())
B.export_pose_table(st, table_out)
doc = {'format': 'reachability-certificate-1', 'seed_table_digest': d0, 'lead_letters': len(lead),
       'safe_all_letters': n_safe, 'safe_all_seed': seed, 'u': u, 'digest_after_each_letter': digests,
       'X_digest': st.digest(), 'X_off': st.off_lattice_count(),
       'evidence': 'exact simulator replay; every letter checked admissible'}
Path(cert_out).write_text(json.dumps(doc, indent=1) + '\n')
print(json.dumps({'X_digest': doc['X_digest'], 'off': doc['X_off'], 'u': len(u), 'sec': round(time.time() - T0)}))
