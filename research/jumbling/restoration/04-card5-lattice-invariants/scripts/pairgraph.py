"""The 'pair' graph on chambers of an orbit with 5-cap signatures: an edge joins two chambers whose signatures share
4 grips and whose sub-signature is shared by exactly these two chambers. Prints the cycle structure and tests given
chamber pairs for adjacency. usage: pairgraph.py <research/jumbling> <orbit> [a,b ...]"""
import sys
from collections import Counter, defaultdict
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
ctx = sim.get_context(); d = ctx.data
oid = np.asarray(d.orbit_id)
o = int(sys.argv[2])
ch = np.flatnonzero(oid == o).tolist()
sig = {p: frozenset(int(c) for c in d.signature_list(p)) for p in ch}
bysub = defaultdict(list)
for p, s in sig.items():
    for c in s:
        bysub[s - {c}].append(p)
adj = defaultdict(set)
for sub, ps in bysub.items():
    if len(ps) == 2:
        a, b = ps
        adj[a].add(b); adj[b].add(a)
print('degrees', Counter(len(adj[p]) for p in ch))
seen, cyc = set(), Counter()
for p in ch:
    if p in seen:
        continue
    stack, comp = [p], []
    seen.add(p)
    while stack:
        x = stack.pop(); comp.append(x)
        for y in adj[x]:
            if y not in seen:
                seen.add(y); stack.append(y)
    cyc[len(comp)] += 1
print('component sizes', dict(cyc))
for pair in sys.argv[3:]:
    a, b = map(int, pair.split(','))
    print(pair, 'adjacent' if b in adj[a] else 'not adjacent', 'shared', len(sig.get(a, set()) & sig.get(b, set())) if a in sig and b in sig else None)
