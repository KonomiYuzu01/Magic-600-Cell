"""Merge macro searches into one correction library: distinct nu values, shortest macro first,
keep a basis of the span (Z2 part and Z5 part). w9 stays entry 0.

usage: build_library.py <scriptdir> <out library.json> <search library.json>..."""
import json
import sys

sys.path.insert(0, sys.argv[1])
from nucoords import Span, vec      # noqa: E402

cands = []
for p in sys.argv[3:]:
    try:
        d = json.load(open(p))
    except FileNotFoundError:
        continue
    cands += d['library']
w9 = next(c for c in cands if c['family'] == 'w9')
rest = sorted((c for c in cands if c['family'] != 'w9' and c['nu']), key=lambda c: (c['length'], c['family']))
span = Span()
lib = []
seen = set()
for c in [w9] + rest:
    key = json.dumps(sorted(c['nu'].items()))
    if key in seen:
        continue
    seen.add(key)
    b2, b5 = vec({int(o): tuple(v) for o, v in c['nu'].items()})
    if span.add(len(lib), b2, b5):
        lib.append({'id': len(lib), 'family': c['family'], 'length': c['length'], 'nu': c['nu'],
                    'records': c['records']})
out = {'format': 'I_a-lattice-correction-library-v1', 'menu': 'I_a', 'macros': lib,
       'rank2': len(span.r2), 'rank5': len(span.r5),
       'candidates_seen': len(cands), 'distinct_nonzero_nu': len(seen) - 1}
json.dump(out, open(sys.argv[2], 'w'))
print(json.dumps({k: v for k, v in out.items() if k != 'macros'}), [(m['family'], m['length'], m['nu']) for m in lib])
