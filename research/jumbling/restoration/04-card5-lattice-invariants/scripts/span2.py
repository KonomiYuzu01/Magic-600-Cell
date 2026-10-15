"""F2 span of verified nu vectors: library macros, named extra vectors and jsonl files of verified words (field
'nu'). Reports the rank over the 43 C2 coordinates (35 signs b0..b34, 8 C2 frames), the C5 coordinates
(17, 28) separately, the touched and untouched coordinates, and the relations satisfied by every vector.
usage: span2.py <library json> <jsonl ...> [-- name=coord,coord ...]"""
import json, sys
import numpy as np
C2F = [0, 3, 6, 8, 11, 22, 25, 32]
names = [f'b{o}' for o in range(35)] + [f'f{o}' for o in C2F]
args = sys.argv[1:]
extras = []
if '--' in args:
    i = args.index('--'); extras = args[i + 1:]; args = args[:i]
L = json.load(open(args[0]))
rows, c5 = [], {}
def vec(nu):
    v = [0] * 43
    for o, (s, c) in nu.items():
        o = int(o)
        v[o] = s & 1
        if o in C2F:
            v[35 + C2F.index(o)] = c & 1
    return v
for m in L['macros']:
    rows.append((f"M{m['id']}", vec(m['nu'])))
for path in args[1:]:
    for k, line in enumerate(open(path)):
        if line.strip():
            r = json.loads(line)
            nm = f"{path.split('/')[-1]}:{k}"
            rows.append((nm, vec(r['nu'])))
            for o in ('17', '28'):
                if o in r['nu'] and r['nu'][o][1] % 5:
                    c5[o] = nm
for extra in extras:
    nm, coords = extra.split('=')
    v = [0] * 43
    for t in coords.split(','):
        v[names.index(t)] = 1
    rows.append((nm, v))
A = np.array([r for _, r in rows], dtype=np.uint8)
R = A.copy(); piv = []; r = 0
for c in range(43):
    p = next((i for i in range(r, len(R)) if R[i, c]), None)
    if p is None: continue
    R[[r, p]] = R[[p, r]]
    for i in range(len(R)):
        if i != r and R[i, c]: R[i] ^= R[r]
    piv.append(c); r += 1
touched = sorted(set(np.flatnonzero(A.any(0)).tolist()))
print('vectors', len(rows), 'rank over C2', r)
print('C5 coordinates with a nonzero generator:', c5 or 'none')
print('untouched C2 coordinates:', [names[c] for c in range(43) if c not in touched])
M2 = A[:, touched].copy(); n = M2.shape[1]
Rm = M2.copy(); pv = []; rr = 0
for c in range(n):
    p = next((i for i in range(rr, len(Rm)) if Rm[i, c]), None)
    if p is None: continue
    Rm[[rr, p]] = Rm[[p, rr]]
    for i in range(len(Rm)):
        if i != rr and Rm[i, c]: Rm[i] ^= Rm[rr]
    pv.append(c); rr += 1
free = [c for c in range(n) if c not in pv]
print('relations on the touched coordinates (each sum vanishes on the span):', len(free))
for f in free:
    y = np.zeros(n, np.uint8); y[f] = 1
    for i, c in enumerate(pv):
        y[c] = Rm[i, f]
    print('  ', '+'.join(names[touched[c]] for c in range(n) if y[c]))
