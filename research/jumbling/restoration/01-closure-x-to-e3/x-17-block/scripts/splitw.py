"""Split a native word into named parts with input and output digests (replayed, every twist checked admissible).

Usage: python -I splitw.py <theory dir> <pose table> <word json> <out dir> <name:length> ...
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import orient as O  # noqa: E402

a = sys.argv[1:]
R, M = O.start(a[1])
word = json.loads(Path(a[2]).read_text())['word']
out = Path(a[3])
out.mkdir(parents=True, exist_ok=True)
pos, rows = 0, []
for spec in a[4:]:
    name, n = spec.split(':')
    n = int(n)
    part = word[pos:pos + n]
    d_in = R.st.digest()
    for c, i in part:
        R.apply(int(c), int(i))
    d_out = R.st.digest()
    (out / f'{name}.json').write_text(json.dumps({'word': part, 'input_digest': d_in, 'output_digest': d_out}) + '\n')
    rows.append({'part': name, 'twists': n, 'input_digest': d_in, 'output_digest': d_out,
                 'off_end': R.st.off_lattice_count()})
    pos += n
if pos != len(word):
    raise SystemExit(f'parts cover {pos} of {len(word)} twists')
print(json.dumps(rows, indent=1))
