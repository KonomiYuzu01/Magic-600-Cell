"""Run the round 11 retained cascade on the committed synthetic input family.

Usage, from the repository root:
  python3 -I -B .../restore_schedule11.py REALIGN.json[.gz] OUT uniform:1 uniform:2

Research evidence only: exact synthetic configurations, with no performance or
Windows/DirectX claim.  The driver and its correction machinery are independent.
"""
import gzip
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
sys.path.insert(0, str(HERE))
from rlib import Retained, NS
from kit import SLP
from sched11 import Schedule11, BOUND

HG = {0: 'C2', 3: 'C2', 8: 'C2', 11: 'C2', 22: 'C2', 25: 'C2', 32: 'C2',
      17: 'C5', 28: 'C5', 6: 'D5', 34: 'A5'}


def ab_maps(base):
    """The same frame groups and abelianisation convention as the round 7 runner."""
    out = {}
    for o in range(35):
        kit = base[o]
        k = kit.k
        if o not in HG:
            out[o] = (None, 1)
            continue
        kit.prepare()
        H = set(kit.H)

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


def uniform_labels(seed, base, AB):
    # Body copied unchanged from restore_realigned.uniform_labels.  In
    # particular, preserve every RNG draw, fibre ordering and parity adjustment.
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


def main(argv=None):
    args = list(sys.argv[1:] if argv is None else argv)
    if len(args) < 3 or any(not s.startswith('uniform:') for s in args[2:]):
        raise SystemExit('usage: restore_schedule11.py REALIGN.json[.gz] OUT uniform:<seed> ...')
    source, OUT, INPUTS = Path(args[0]), Path(args[1]), args[2:]
    with (gzip.open(source, 'rt') if source.suffix == '.gz' else source.open()) as f:
        RJ = json.load(f)
    T0 = time.time()
    R = Retained('.')
    slp = SLP(R)
    schedule = Schedule11(R, slp, RJ)
    cascade, base = schedule.cascade, schedule.kits
    AB = ab_maps(base)
    REAL = {d['orbit']: d for d in RJ['orbits']}
    COMMITTED = json.loads((S03 / 'restore' / 'restore-summary.json').read_text())
    assert cascade == COMMITTED['cascade_order']
    DIGEST = {r['input']: r['input_labels_sha256'] for r in COMMITTED['runs'] if 'input_labels_sha256' in r}
    LIBJ = S03 / 'restore' / 'library-final.json'
    summary = {'cascade_order': cascade, 'replaced_orbits': sorted(o for o in REAL if REAL[o]['replaced']),
               'seed_lengths': {str(o): REAL[o]['s'] for o in range(35)},
               'library': {'sha256': hashlib.sha256(LIBJ.read_bytes()).hexdigest(), 'used': False},
               'bound': BOUND, 'runs': []}
    results, reports = [], []
    OUT.mkdir(parents=True, exist_ok=True)
    for spec in INPUTS:
        t0 = time.time()
        L = uniform_labels(int(spec.split(':')[1]), base, AB)
        rec = {'input': spec, 'input_labels_sha256': hashlib.sha256(L.astype('<i8').tobytes()).hexdigest()}
        rec['input_matches_committed'] = rec['input_labels_sha256'] == DIGEST.get(spec)
        rec['moved_pieces_input'] = int(len(np.unique(R.sp[np.flatnonzero(L != np.arange(NS))])))
        rec['corrections'], rec['w9'], rec['bound'] = [], False, BOUND
        L0 = L.copy()
        factors, per, per_len = [], {}, {}
        for o in cascade:
            f = schedule.solve_orbit(o, L)
            per[o], per_len[o] = len(f), int(sum(slp.length[i] for i, _ in f))
            factors += f
        assert np.array_equal(L, np.arange(NS)), 'not restored'
        bounds = schedule.orbit_bounds()
        assert sum(bounds.values()) == BOUND, ('computed tariffs differ', sum(bounds.values()))
        assert all(per_len[o] <= bounds[o] for o in cascade), 'per-orbit bound exceeded'
        rec['status'] = 'restored'
        rec['factors'] = len(factors)
        rec['factors_per_orbit'] = {str(o): per[o] for o in cascade}
        rec['length_per_orbit'] = {str(o): per_len[o] for o in cascade}
        rec['method_per_orbit'] = {str(o): schedule.stats[o]['method'] for o in cascade}
        rec['counts_per_orbit'] = {str(o): schedule.stats[o]['counts'] for o in cascade}
        rec['retained_length'] = int(sum(per_len.values()))
        assert rec['retained_length'] <= BOUND
        rec['seconds'] = round(time.time() - t0, 1)
        summary['runs'].append(rec)
        tag = spec.replace(':', '_').replace('/', '_')[-60:]
        np.save(OUT / ('start-labels-%s.npy' % tag), L0)
        results.append((tag, factors))
        ratio, orbit = max((per_len[o] / bounds[o], o) for o in cascade)
        reports.append('%-16s total %10d  factors %7d  largest length/bound %.6f (orbit %d)  %.1fs'
                       % (spec, rec['retained_length'], len(factors), ratio, orbit, rec['seconds']))
        print(reports[-1], flush=True)

    # Preserve the stage 03 serialization, including its dependency ordering.
    used, stack = set(), [i for _, fs in results for i, _ in fs]
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
    print('done %.1fs nodes %d' % (time.time() - T0, len(keep)), flush=True)


if __name__ == '__main__':
    main()
