"""Run the committed lattice cascade of stage 03 with the replacement seeds that `realign.py` certified
(SENIOR-REVIEW.md section 12).

The kits, the order, the labels and the synthetic inputs are those of stage 03 `restore_lattice.py` (lines 49 to 70
and 223 to 256, reproduced here without its correction machinery, which `uniform:<seed>` inputs do not need). On a
replaced orbit the kit's seed c is the certified word, and `pair_to` re-indexes the pairs: it takes the pair at B
whose conjugate has the required value at the target position, instead of assuming that c is aligned with the
ordered fibres. The other orbits keep the committed kit unchanged. Each input is checked against the committed
input digest, solved, and written as stage 03 writes it (restore-summary.json, solutions.json, start labels), so
that the stage 03 replay checker `checker/verify_restore.py` replays it with its own code.
usage (from the repository root): python3 -I -B restore_realigned.py realign.json OUT_DIR uniform:1 [uniform:2 ...]
Research only: exact synthetic geometry; no Windows, DirectX, input or performance claim.
"""
import hashlib
import json
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
GM = HERE.parents[1]
S03 = GM.parent / '03-retained-group-and-lattice-restorer'
sys.path.insert(0, str(S03 / 'scripts'))
from rlib import Retained, NS             # noqa: E402
from kit import SLP, OrbitKit             # noqa: E402

RJ, OUT, INPUTS = json.loads(Path(sys.argv[1]).read_text()), Path(sys.argv[2]), sys.argv[3:]
T0 = time.time()
R = Retained('.')
HG = {0: 'C2', 3: 'C2', 8: 'C2', 11: 'C2', 22: 'C2', 25: 'C2', 32: 'C2', 17: 'C5', 28: 'C5', 6: 'D5', 34: 'A5'}
REAL = {d['orbit']: d for d in RJ['orbits']}
COMMITTED = json.loads((S03 / 'restore' / 'restore-summary.json').read_text())
DIGEST = {r['input']: r['input_labels_sha256'] for r in COMMITTED['runs'] if 'input_labels_sha256' in r}


class ReKit(OrbitKit):
    """OrbitKit for a seed that need not be aligned with the ordered fibres: `pair_to` searches the pairs at B for
    the one whose conjugate has value v at X, and checks the value exactly."""

    def pair_to(self, v, X):
        self._pairs()
        if X == self.B:
            q, sg = self.pair_by_value[v]
            node = q if sg > 0 else self.slp.prod([(q, -1)])
        else:
            node = None
            for q, sg in self.pair_by_value.values():
                base = q if sg > 0 else self.slp.prod([(q, -1)])
                cand = (self.slp.conj(base, self.star(self.some_other()), -1) if X == self.A
                        else self.slp.conj(base, self.star(X), 1))
                if self.value_at(self.slp.net[cand], X) == v:
                    node = cand
                    break
            assert node is not None, (self.o, v, X)
        assert self.value_at(self.slp.net[node], X) == v
        return node


# ---- kits (stage 03 restore_lattice.py lines 49 to 70) --------------------------------------
slp = SLP(R)
coll, seed_node = {}, {}
for o in range(35):
    seed_node[o] = slp.word(R.trees[o]['seed'])
    coll[o] = sorted(set(int(x) for x in R.so[slp.net[seed_node[o]][0]]) - {o})
order, done = [], set()
while len(order) < 35:
    for o in [o for o in range(35) if o not in done and set(coll[o]) <= done]:
        order.append(o)
        done.add(o)
cascade = order[::-1]
assert cascade == RJ['cascade_order'] == COMMITTED['cascade_order']
after = {o: set(cascade[i + 1:]) for i, o in enumerate(cascade)}
base, kits = {}, {}
for o in cascade:
    w = [int(m) for m in R.trees[o]['relocation']]
    reloc = slp._add({'op': 'word', 'w': w}, (np.zeros(0, np.int64), np.zeros(0, np.int64)), len(w))
    slp.word_of[reloc] = w
    c = slp.conj(seed_node[o], reloc)
    fa, fb, fc = [np.array(f, np.int64) for f in R.atlas[o]['frames']]
    for a, b in ((fa, fb), (fb, fc), (fc, fa)):
        assert np.array_equal(SLP.apply_sparse(a, slp.net[c]), b)
    base[o] = OrbitKit(R, slp, o, c, allowed=set(coll[o]), compact_paths=True)
    d = REAL[o]
    if not d['replaced']:
        kits[o] = base[o]
        continue
    ch = d['choice']
    c2 = slp.word(ch['word'])
    assert slp.length[c2] == d['s'] == len(ch['word'])
    s, dd = slp.net[c2]
    col = set(int(x) for x in R.so[s]) - {o}
    assert col == set(ch['collateral']) and col <= after[o], (o, col)
    A, B, C = base[o].A, base[o].B, base[o].C
    on = s[R.so[s] == o]
    assert set(R.sp[on].tolist()) == {A, B, C}, o
    for x, y in ((A, B), (B, C), (C, A)):
        img = SLP.apply_sparse(base[o].lab[x][:1], slp.net[c2])
        assert int(R.sp[img[0]]) == y, (o, x, y)
    kits[o] = ReKit(R, slp, o, c2, allowed=col, compact_paths=True)
print('kits %.1fs' % (time.time() - T0), flush=True)


# ---- synthetic inputs (stage 03 restore_lattice.py lines 85 to 128 and 223 to 256) -----------
def ab_maps():
    out = {}
    for o in range(35):
        kit = base[o]
        k = kit.k
        if o not in HG:
            out[o] = (None, 1)
            continue
        kit._pairs()
        vals = list(kit.pair_by_value)
        H = set(vals)
        fr = list(vals)
        while fr:
            nx = []
            for x in fr:
                for g in vals:
                    y = tuple(g[x[i]] for i in range(k))
                    if y not in H:
                        H.add(y)
                        nx.append(y)
            fr = nx

        def order_of(x):
            e, y, n = tuple(range(k)), x, 1
            while y != e:
                y, n = tuple(x[y[i]] for i in range(k)), n + 1
            return n
        if HG[o] == 'C2':
            out[o] = ({h: int(h != tuple(range(k))) for h in H}, 2)
        elif HG[o] == 'D5':
            out[o] = ({h: int(order_of(h) == 2) for h in H}, 2)
        elif HG[o] == 'C5':
            r0 = min(h for h in H if order_of(h) == 5)
            m, x = {}, tuple(range(k))
            for i in range(5):
                m[x] = i
                x = tuple(r0[x[j]] for j in range(k))
            out[o] = (m, 5)
        else:
            out[o] = ({h: 0 for h in H}, 1)
    return out


AB = ab_maps()


def uniform_labels(seed):
    rng = np.random.default_rng(seed)
    L = np.arange(NS, dtype=np.int64)
    for o in range(35):
        kit = base[o]
        P = np.array(sorted(kit.lab))
        img = rng.permutation(len(P))
        seen, par = np.zeros(len(P), bool), 0
        for i in range(len(P)):
            if seen[i]:
                continue
            n, j = 0, i
            while not seen[j]:
                seen[j] = True
                j = img[j]
                n += 1
            par ^= (n - 1) & 1
        if par:
            img[[0, 1]] = img[[1, 0]]
        tab, mod = AB[o]
        H = sorted(tab) if tab is not None else [tuple(range(kit.k))]
        frames = [H[i] for i in rng.integers(0, len(H), len(P))]
        if tab is not None:
            tot = sum(tab[h] for h in frames[:-1]) % mod
            need = (-tot) % mod
            frames[-1] = next(h for h in H if tab[h] == need)
        for i, Y in enumerate(P):
            X = int(P[img[i]])
            v = frames[i]
            for t in range(kit.k):
                L[kit.lab[X][v[t]]] = kit.lab[Y][t]
    return L


def run(spec):
    assert spec.startswith('uniform:'), 'only synthetic inputs: ' + spec
    t0 = time.time()
    L = uniform_labels(int(spec.split(':')[1]))
    rec = {'input': spec, 'input_labels_sha256': hashlib.sha256(L.astype('<i8').tobytes()).hexdigest()}
    rec['input_matches_committed'] = rec['input_labels_sha256'] == DIGEST.get(spec)
    rec['moved_pieces_input'] = int(len(np.unique(R.sp[np.flatnonzero(L != np.arange(NS))])))
    rec['corrections'], rec['w9'] = [], False
    L0 = L.copy()
    factors, per, per_len = [], {}, {}
    for o in cascade:
        f = kits[o].solve(L)
        per[o], per_len[o] = len(f), int(sum(slp.length[i] for i, _ in f))
        factors += f
    assert np.array_equal(L, np.arange(NS)), 'not restored'
    rec['status'] = 'restored'
    rec['factors'] = len(factors)
    rec['factors_per_orbit'] = {str(o): per[o] for o in cascade}
    rec['length_per_orbit'] = {str(o): per_len[o] for o in cascade}
    rec['retained_length'] = int(sum(per_len.values()))
    rec['seconds'] = round(time.time() - t0, 1)
    return rec, (L0, factors)


OUT.mkdir(parents=True, exist_ok=True)
LIBJ = S03 / 'restore' / 'library-final.json'
summary = {'cascade_order': cascade, 'replaced_orbits': sorted(o for o in REAL if REAL[o]['replaced']),
           'seed_lengths': {str(o): REAL[o]['s'] for o in range(35)},
           # the stage 03 checker checks this digest; synthetic inputs use no correction from the library
           'library': {'sha256': hashlib.sha256(LIBJ.read_bytes()).hexdigest(), 'used': False}, 'runs': []}
results = []
for spec in INPUTS:
    rec, sol = run(spec)
    print(json.dumps({k: v for k, v in rec.items() if k not in ('factors_per_orbit', 'length_per_orbit')}), flush=True)
    summary['runs'].append(rec)
    tag = spec.replace(':', '_').replace('/', '_')[-60:]
    np.save(OUT / ('start-labels-%s.npy' % tag), sol[0])
    results.append((tag, sol[1]))
# one SLP for all runs (stage 03 restore_lattice.py lines 366 to 395)
used = set()
stack = [i for _, fs in results for i, _ in fs]
while stack:
    i = stack.pop()
    if i in used:
        continue
    used.add(i)
    nd = slp.nodes[i]
    if nd['op'] == 'conj':
        stack += [nd['x'], nd['u']]
    elif nd['op'] == 'prod':
        stack += [f[0] for f in nd['f']]
keep = sorted(used)
remap = {old: new for new, old in enumerate(keep)}


def rn(nd):
    if nd['op'] in ('word', 'path'):
        return nd
    if nd['op'] == 'conj':
        return {'op': 'conj', 'x': remap[nd['x']], 'u': remap[nd['u']], 'us': nd['us']}
    return {'op': 'prod', 'f': [[remap[a], b] for a, b in nd['f']]}


doc = {'format': 'lattice-restoration-v1', 'model_files': R.files,
       'conventions': 'as card1-certificates-v1, plus {"op":"path","orbit":o,"node":n}: the execution-tree path from the root to node n',
       'nodes': [rn(slp.nodes[i]) for i in keep],
       'solutions': {tag: [[remap[i], sg] for i, sg in fs] for tag, fs in results}}
(OUT / 'solutions.json').write_text(json.dumps(doc))
(OUT / 'restore-summary.json').write_text(json.dumps(summary, indent=1))
print('done %.1fs nodes %d' % (time.time() - T0, len(keep)))
