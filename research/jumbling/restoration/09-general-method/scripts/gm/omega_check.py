"""Theorem-specific check for a pure twist (Lemma C, senior review LC-11): compare Bad(W) with Omega_X.

Bad(W) depends on the word alone: the ghost of every home chamber in the union of the word's caps (occupied or
vacant) is moved through W, and a chamber is bad when its ghost dies or ends off the lattice. Omega_X is the set of
home chambers whose class-P cap signature is exactly the cell X. The word W is a slice of a stored step word (for a
closure V W V^-1, the slice after V).

usage: omega_check.py <theory dir> <echain dir> <word json> <offset> <length> <class> <out json> <cap> <cap>...
Headless research only; not Windows, DirectX or performance evidence.
"""
import hashlib
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import classes as CL  # noqa: E402
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402

word_file, off, n, P, out = sys.argv[3], int(sys.argv[4]), int(sys.argv[5]), int(sys.argv[6]), sys.argv[7]
X = sorted(int(c) for c in sys.argv[8:])
full = json.loads(Path(word_file).read_text())
W = [tuple(x) for x in full['word'][off:off + n]]
assert len(W) == n, 'slice outside the word'
M = GL.restore.menu('I_a')
chambers = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in sorted({c for c, _ in W})]))
g = FG.Ghost(FG.Frames(), np.zeros(len(chambers), np.int64), chambers)
for c, i in W:
    g.apply(c, GL.restore.element(M, c, i), (c, i))
bad = sorted(chambers[~g.alive | (g.dom != 0)].tolist())
members, _ = CL.classes()
poles = list(members[P])
assert all(c in poles for c in X), 'a cap of X is not in the class'
want = np.array([c in X for c in poles])
ps = np.arange(GL.NP)
sig = np.stack([GL.data.in_sig(ps, np.full(GL.NP, c, np.int64)) for c in poles], 1)
omega = np.flatnonzero((sig == want).all(1)).tolist()


def digest(v):
    return hashlib.sha256(json.dumps(v).encode()).hexdigest()


rec = {'word_file': Path(word_file).name, 'input_digest': full.get('input_digest'), 'offset': off, 'length': n,
       'class': P, 'cell': X, 'chambers_ghosted': int(len(chambers)), 'badW_size': len(bad),
       'omega_X_size': len(omega), 'equal': bad == omega, 'badW_sha256': digest(bad), 'omega_X_sha256': digest(omega),
       'badW': bad, 'omega_X': omega}
Path(out).write_text(json.dumps(rec, indent=1) + '\n')
print(json.dumps({k: v for k, v in rec.items() if k not in ('badW', 'omega_X')}))
sys.exit(0 if rec['equal'] else 1)
