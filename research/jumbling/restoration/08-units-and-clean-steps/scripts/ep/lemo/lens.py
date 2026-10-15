"""Parking lens caps of a unit's family words (certificate input, menu I_a).

dfam2.py parks a word Q (W = P Q P^-1, P = [[X, Y], Z]) only if some cap p != c holds all of Bad(Q) (a 'lens' cap,
where Y acts), and only for Q's with Bad(Q) at most twice the smallest Bad of the words usable at the state. For the
family rows given (D-type and C-type rows of dfam2, every parameter) whose ghost lands T and whose Bad size is at most
--max-bad, this script computes Bad(Q) exactly (frame ghost of the lattice chambers of Q's caps) and every cap that
holds all of it. A word with no lens cap other than its grip c has no parking word in any state (the other parking
conditions, admissibility and poles, only remove candidates).

usage: lens.py <theory> <echain> <table> <q.jsonl[.gz]>... --out <json> --max-bad N   (every word with a lens is listed)
"""
import gzip
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import ghostlib as GL  # noqa: E402

a = sys.argv[3:]
out = a[a.index('--out') + 1]
MAXB = int(a[a.index('--max-bad') + 1])
files = [x for x in a[1:a.index('--out')]] if a.index('--out') > 1 else []
table = a[0]
C = GL.Ctx(table)
data = GL.data
rows = []
for f in files:
    op = gzip.open if f.endswith('.gz') else open
    rows += [json.loads(x) for x in op(f, 'rt') if x.strip()]
tmax = max(r['T_on'] for r in rows)
sel = [r for r in rows if r['T_dead'] == 0 and r['T_on'] == tmax and r['badQ'] <= MAXB]
with_lens, examples, checked, mism = 0, [], 0, 0
for n, r in enumerate(sel):
    Q = [(int(c), int(i)) for c, i in r['Q']]
    bad, _, _ = C.bad(Q)
    checked += 1
    mism += int(len(bad) != r['badQ'])
    sig = data.sig_matrix(bad)
    lens = [int(p) for p in np.flatnonzero(sig.sum(axis=0) == len(bad)) if int(p) != int(r['c'])]
    if lens:
        with_lens += 1
        if len(examples) < 100000:
            examples.append({'Q': r['Q'], 'badQ': r['badQ'], 'lens': lens})
    if n % 200 == 0:
        print(json.dumps({'n': n, 'of': len(sel), 'with_lens': with_lens, 'bad_size_mismatch': mism}), flush=True)
doc = {'files': [Path(f).name for f in files], 'landing_T_on': tmax, 'max_bad': MAXB, 'rows': len(rows),
       'selected': len(sel), 'checked': checked, 'bad_size_mismatch': mism, 'with_lens': with_lens,
       'with_lens_words': examples}
Path(out).write_text(json.dumps(doc) + '\n')
print(json.dumps({k: v for k, v in doc.items() if k != 'with_lens_words'}))
