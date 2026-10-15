"""Replay word files from a pose table and export the current-only pose table at the end (every twist checked).

Usage: python -I exportat.py <theory dir> <pose table> <out table> <word json> ...
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import buffer as B  # noqa: E402

a = sys.argv[1:]
R, M = B.O.start(a[1])
for name in a[3:]:
    doc = json.loads(Path(name).read_text())
    if doc.get('input_digest') not in (None, R.st.digest()):
        raise AssertionError(f'{name}: input digest mismatch')
    for c, i in doc['word']:
        R.apply(int(c), int(i))
    if doc.get('output_digest') not in (None, R.st.digest()):
        raise AssertionError(f'{name}: output digest mismatch')
B.export_pose_table(R.st, a[2])
print(json.dumps({'digest': R.st.digest(), 'off': R.st.off_lattice_count()}))
