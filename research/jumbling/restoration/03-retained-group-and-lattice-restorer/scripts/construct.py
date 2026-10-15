"""Construct the card-1 certificates P_o (35 orbits) and F_o (11 orbits) as an SLP.

usage: construct.py <scriptdir> <repo root> <out dir>
"""
import json
import sys
import time

import numpy as np

sys.path.insert(0, sys.argv[1])
from rlib import Retained, NS          # noqa: E402
from kit import SLP, OrbitKit          # noqa: E402

R = Retained(sys.argv[2])
OUT = sys.argv[3]
slp = SLP(R)
T0 = time.time()

HGROUP = {0: 'C2', 3: 'C2', 8: 'C2', 11: 'C2', 22: 'C2', 25: 'C2', 32: 'C2',
          17: 'C5', 28: 'C5', 6: 'D5', 34: 'A5'}

# collateral orbits of every seed
seed_node, coll = {}, {}
for o in range(35):
    w = R.trees[o]['seed']
    seed_node[o] = slp.word(w)
    s, _ = slp.net[seed_node[o]]
    coll[o] = sorted(set(int(x) for x in R.so[s]) - {o})

order, done = [], set()
while len(order) < 35:
    ready = [o for o in range(35) if o not in done and set(coll[o]) <= done]
    assert ready, 'collateral dependencies are cyclic'
    for o in ready:
        order.append(o)
        done.add(o)
print('order', order, flush=True)

kits, report = {}, {}
for o in order:
    t0 = time.time()
    s, d = slp.net[seed_node[o]]
    L = np.arange(NS, dtype=np.int64)
    L[d] = s
    factors = []
    for q in coll[o]:
        factors += kits[q].solve(L)
    tgt = R.so == o
    rest = np.flatnonzero(L != np.arange(NS))
    assert np.all(tgt[rest]), 'cleanup left stickers outside orbit %d' % o
    raw = slp.prod([(seed_node[o], 1)] + factors) if factors else seed_node[o]
    rs, rd = slp.net[raw]
    assert np.all(R.so[rs] == o)
    reloc = slp._add({'op': 'word', 'w': [int(m) for m in R.trees[o]['relocation']]},
                     (np.zeros(0, np.int64), np.zeros(0, np.int64)), len(R.trees[o]['relocation']))
    slp.word_of[reloc] = [int(m) for m in R.trees[o]['relocation']]
    c = slp.conj(raw, reloc)
    cs, cd = slp.net[c]
    at = R.atlas[o]
    fa, fb, fc = [np.array(f, np.int64) for f in at['frames']]
    for a, b in ((fa, fb), (fb, fc), (fc, fa)):
        assert np.array_equal(SLP.apply_sparse(a, (cs, cd)), b), 'frame cycle mismatch on %d' % o
    assert len(cs) == 3 * len(fa)
    kit = OrbitKit(R, slp, o, c)
    kits[o] = kit
    report[o] = dict(orbit=o, seed_length=len(R.trees[o]['seed']), collateral=coll[o],
                     cleanup_factors=len(factors), c_node=c, c_length=slp.length[c],
                     secs=round(time.time() - t0, 1))
    print(o, 'coll', len(coll[o]), 'factors', len(factors), 'len(c)', slp.length[c],
          '%.1fs' % (time.time() - t0), 'nodes', len(slp.nodes), flush=True)

# F_o
GROUP_ORDER = {'C2': 2, 'C5': 5, 'D5': 10, 'A5': 60}


def pcompose(x, y):          # chronological: x first, then y
    return tuple(y[x[i]] for i in range(len(x)))


def pinv(x):
    r = [0] * len(x)
    for i, v in enumerate(x):
        r[v] = i
    return tuple(r)


def porder(x):
    e = tuple(range(len(x)))
    n, y = 1, x
    while y != e:
        y = pcompose(y, x)
        n += 1
    return n


def closure(gens, k):
    e = tuple(range(k))
    seen = {e}
    fr = [e]
    while fr:
        nx = []
        for x in fr:
            for g in gens:
                y = pcompose(x, g)
                if y not in seen:
                    seen.add(y)
                    nx.append(y)
        fr = nx
    return seen


fcert = {}
for o, hg in sorted(HGROUP.items()):
    kit = kits[o]
    kit._pairs()
    k = kit.k
    e = tuple(range(k))
    H = closure(list(kit.pair_by_value), k)
    assert len(H) == GROUP_ORDER[hg], (o, hg, len(H))
    cands = []
    pairs = sorted(set(kit.pair_by_value.values()))
    for (q1, s1) in pairs:
        cands.append([(q1, s1)])
    for (q1, s1) in pairs:
        for (q2, s2) in pairs:
            cands.append([(q1, s1), (q2, s2)])
    found = None
    for fs in cands:
        node = slp.prod(fs) if len(fs) > 1 or fs[0][1] < 0 else fs[0][0]
        net = slp.net[node]
        assert set(R.sp[net[0]].tolist()) <= {kit.B, kit.C}
        vB, vC = kit.value_at(net, kit.B), kit.value_at(net, kit.C)
        ok = None
        if hg == 'C2':
            ok = vB != e and vC != e and (vB, vC, kit.B, kit.C)
        elif hg == 'C5':
            ok = porder(vB) == 5 and vC == pinv(vB) and (vB, vC, kit.B, kit.C)
        elif hg == 'D5':
            if porder(vB) == 2 and porder(vC) == 2 and vB != vC:
                ok = (vB, vC, kit.B, kit.C)
        elif hg == 'A5':
            for (x, y, i, j) in ((vB, vC, kit.B, kit.C), (vC, vB, kit.C, kit.B)):
                if porder(x) == 2 and porder(y) == 3 and len(closure([x, y], k)) == 60:
                    ok = (x, y, i, j)
                    break
        if ok:
            found = (node, ok)
            break
    if found is None and hg == 'A5':
        # two-pair products on {B, C} give conjugate values of equal order; use two singles
        invs = [v for v in H if porder(v) == 2]
        thr = [v for v in H if porder(v) == 3]
        pick = next((a, b) for a in sorted(invs) for b in sorted(thr) if len(closure([a, b], k)) == 60)
        a, b = pick
        sc_b = kit.single_at_C(b)
        sc_a = kit.single_at_C(a)
        sb_a = slp.conj(sc_a, kit.c, -1)          # carried by c^-1: C -> B
        node = slp.prod([(sb_a, 1), (sc_b, 1)])
        net = slp.net[node]
        assert set(R.sp[net[0]].tolist()) == {kit.B, kit.C}
        vB, vC = kit.value_at(net, kit.B), kit.value_at(net, kit.C)
        assert vB == a and vC == b
        found = (node, (vB, vC, kit.B, kit.C))
    assert found, 'no F certificate for orbit %d' % o
    node, (x, y, i, j) = found
    fcert[o] = dict(orbit=o, group=hg, node=node, i=int(i), j=int(j), value_i=list(x), value_j=list(y),
                    length=slp.length[node])
    print('F', o, hg, 'node', node, 'values', x, y, 'len', slp.length[node], flush=True)

# serialise
nodes = slp.nodes
used = set()
stack = [report[o]['c_node'] for o in range(35)] + [fcert[o]['node'] for o in fcert]
while stack:
    i = stack.pop()
    if i in used:
        continue
    used.add(i)
    nd = nodes[i]
    if nd['op'] == 'conj':
        stack += [nd['x'], nd['u']]
    elif nd['op'] == 'prod':
        stack += [f[0] for f in nd['f']]
keep = sorted(used)
remap = {old: new for new, old in enumerate(keep)}


def rn(nd):
    if nd['op'] == 'word':
        return nd
    if nd['op'] == 'conj':
        return {'op': 'conj', 'x': remap[nd['x']], 'u': remap[nd['u']], 'us': nd['us']}
    return {'op': 'prod', 'f': [[remap[a], b] for a, b in nd['f']]}


out_nodes = [rn(nodes[i]) for i in keep]
pcert = {}
for o in range(35):
    kit = kits[o]
    at = R.atlas[o]
    setups = {str(X): int(n) for X, n in sorted(kit.node_at.items()) if X != kit.C}
    pcert[o] = dict(orbit=o, A=kit.A, B=kit.B, C=kit.C, frames=[at['frames'][0], at['frames'][1], at['frames'][2]],
                    c_node=remap[report[o]['c_node']], c_length=report[o]['c_length'],
                    setup_tree_nodes=setups, k=kit.k, N=len(kit.lab))
for o in fcert:
    fcert[o]['node'] = remap[fcert[o]['node']]
doc = dict(format='card1-certificates-v1', model_files=R.files,
           conventions='words are signed primitive ids (+-1..+-1200) applied left to right; conj(x,u,us) = (u^us)^-1 x (u^us) chronologically; prod f1 f2 ... applies f1 first; setups are execution-tree paths from the root (node 0) of assets/execution_trees.json.gz',
           nodes=out_nodes, P=[pcert[o] for o in range(35)], F=[fcert[o] for o in sorted(fcert)],
           construction=dict(order=order, report=[report[o] for o in range(35)], seconds=round(time.time() - T0, 1)))
with open(OUT + '/certificates.json', 'w') as f:
    json.dump(doc, f)
print('nodes kept', len(out_nodes), 'of', len(nodes), 'total %.1fs' % (time.time() - T0))
