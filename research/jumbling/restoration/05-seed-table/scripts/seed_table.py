"""The 3E seed table (menu I_a): one row per position class (S'-orbit of positions, 500 classes), with its size,
registry orbit, model orbit of its chambers, core status, membership in the harvest reach bound, presence among the
off pieces of the deep sample fixture40 and of the frozen inputs, and the seeds of the library (count, shortest
seed, distinct grip-set footprints).
usage: seed_table.py <research/jumbling> <classes.json> <arrays.npz> <seeds json> <reach json> <out json>
                     <pipeline run json> ..."""
import json, sys
from collections import Counter, defaultdict
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from e3lib import REGNAMES, Lib  # noqa: E402
L = Lib(sys.argv[1], sys.argv[2], sys.argv[3])
oid = np.asarray(L.d.orbit_id)
seeds = json.load(open(sys.argv[4]))
reach = set(json.load(open(sys.argv[5]))['union'])
present = defaultdict(Counter)
for rp in sys.argv[7:]:
    run = json.load(open(rp))
    for p in run['pieces']:
        if p.get('depth_one'):
            present[run['summary']['input']][p['class']] += 1
by = defaultdict(list)
for q in seeds:
    by[q['class']].append(q)
rows = []
for c in sorted(set(L.cls_of.tolist())):
    qs = sorted(by.get(c, []), key=lambda q: q['length'])
    rows.append({'class': c, 'size': int(L.csize[c]), 'registry_orbit': REGNAMES[int(L.reg_code[c])],
                 'model_orbit': int(oid[c]), 'core_forbidden': bool(L.core_bad[c]), 'in_harvest_bound': c in reach,
                 'present_in': {k: v[c] for k, v in present.items() if v.get(c)},
                 'seeds': len(qs), 'shortest_seed': qs[0]['name'] if qs else None,
                 'shortest_length': qs[0]['length'] if qs else None,
                 'footprints': len({tuple(q['grips']) for q in qs})})
summ = {'classes': len(rows), 'core_ok': sum(not r['core_forbidden'] for r in rows),
        'in_harvest_bound': sum(r['in_harvest_bound'] for r in rows),
        'with_seed': sum(r['seeds'] > 0 for r in rows),
        'with_3_footprints': sum(r['footprints'] >= 3 for r in rows),
        'by_registry': {}}
for rg in REGNAMES:
    rr = [r for r in rows if r['registry_orbit'] == rg]
    summ['by_registry'][rg] = {'classes': len(rr), 'core_ok': sum(not r['core_forbidden'] for r in rr),
                               'in_bound': sum(r['in_harvest_bound'] for r in rr),
                               'with_seed': sum(r['seeds'] > 0 for r in rr)}
for name in present:
    cs = set(present[name])
    summ[f'classes_present_{name}'] = len(cs)
    summ[f'classes_present_with_seed_{name}'] = sum(1 for c in cs if by.get(c))
json.dump({'summary': summ, 'rows': rows}, open(sys.argv[6], 'w'), indent=1)
print(json.dumps(summ, indent=1))
