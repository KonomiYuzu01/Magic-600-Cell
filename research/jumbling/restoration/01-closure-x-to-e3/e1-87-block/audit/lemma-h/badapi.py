"""Audit lemma-h: Bad(U) through the literal API ghostlib.Ctx(table).bad(word), timed.

Usage: python3 -I badapi.py <theory dir> <v26 dir> <table> <word json> <start> <end> <compare json> <field>
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
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import ghostlib as GL  # noqa: E402

t0 = time.time()
C = GL.Ctx(sys.argv[3])
t1 = time.time()
word = [(int(c), int(i)) for c, i in json.loads(Path(sys.argv[4]).read_text())['word']][int(sys.argv[5]):int(sys.argv[6])]
bad, g, ch = C.bad(word)
t2 = time.time()
ref = json.loads(Path(sys.argv[7]).read_text())[sys.argv[8]]
holes = np.flatnonzero(~C.occ)
print(json.dumps({'twists': len(word), 'chambers': int(len(ch)), 'bad_size': int(len(bad)),
                  'equals_earlier_run': sorted(map(int, bad)) == sorted(ref),
                  'subset_holes': bool(np.isin(bad, holes).all()), 'equals_holes': bool(np.array_equal(np.sort(bad), holes)),
                  'load_s': round(t1 - t0, 1), 'bad_s': round(t2 - t1, 1)}))
