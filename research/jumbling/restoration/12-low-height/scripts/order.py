# Landing order under the structural model of the screen (screen.py output): a unit can be landed by a T1 word once
# every grip it needs is free, that is once the units that straddle or foreign-occupy those grips have been landed
# (for a set of alternative grips, once one of them is clear), and only when its cell holds no second frame domain of
# its class (M1-19). Starting from the units whose needed grips are free at the state, land in rounds until nothing
# changes. Lemma D, N1 and N2 are not re-evaluated along the way, and no word is run: this is a structural bound.
# The output also counts the units that pass the screen's necessary T1 checks at the state itself (T1_necessary).
# The units left over are split into those in an M1-19 cell, those in a group that blocks itself (a strongly
# connected component of the blocking relation among the units left over), and those only behind such units.
# Usage: python3 order.py <screen json>
import json, sys
from collections import Counter, defaultdict

d = json.load(open(sys.argv[1]))
U = {u['unit']: u for u in d['units']}


def need(u):
    g = u.get('grip_status', {})
    blk = lambda c: set(g[str(c)]['straddled_by']) | set(g[str(c)]['foreign'])
    return [blk(c) for c in u['needed']['all']], [[blk(c) for c in s] for s in u['needed']['one_of']]


def eligible(u):
    return u.get('class') is not None and u.get('kind') not in (None, 'outer') and not u.get('other_domains_in_cell')


landed, rounds = set(), []
while True:
    new = []
    for k, u in U.items():
        if k in landed or not eligible(u):
            continue
        conj, disj = need(u)
        if all(b <= landed for b in conj) and all(any(b <= landed for b in s) for s in disj):
            new.append(k)
    if not new:
        break
    landed |= set(new)
    rounds.append(len(new))
L = set(U) - landed
edges = set()
for b in L:
    if not eligible(U[b]):
        continue
    conj, disj = need(U[b])
    for s in conj:
        edges |= {(a, b) for a in s & L}
    for alts in disj:
        if all(s & L for s in alts):
            for s in alts:
                edges |= {(a, b) for a in s & L}
out, inn = defaultdict(set), defaultdict(set)
for a, b in edges:
    out[a].add(b)
    inn[b].add(a)


def dfs(v, g, seen, acc):
    stack = [(v, iter(sorted(g[v])))]
    seen.add(v)
    while stack:
        x, it = stack[-1]
        for y in it:
            if y not in seen:
                seen.add(y)
                stack.append((y, iter(sorted(g[y]))))
                break
        else:
            stack.pop()
            acc.append(x)


fin, seen = [], set()
for v in sorted(L):
    if v not in seen:
        dfs(v, out, seen, fin)
comps, seen = [], set()
for v in reversed(fin):
    if v not in seen:
        acc = []
        dfs(v, inn, seen, acc)
        comps.append(sorted(acc))
groups = [c for c in comps if len(c) > 1 or (c[0], c[0]) in edges]
ing = {x for c in groups for x in c}
m119 = {b for b in L if U[b].get('other_domains_in_cell')}
behind = L - ing - m119
size = lambda S: sum(U[k]['size'] for k in S)
print(json.dumps({
    'units': len(U), 'pieces': size(U), 'landable_in_order': len(landed), 'pieces_landable': size(landed),
    'rounds': rounds, 'left': len(L), 'pieces_left': size(L),
    'left_in_m1_19_cells': len(m119), 'pieces_in_m1_19_cells': size(m119),
    'left_in_blocking_groups': len(ing), 'pieces_in_blocking_groups': size(ing),
    'blocking_groups': [{'units': c, 'pieces': size(c)} for c in groups],
    'left_behind_them': len(behind), 'pieces_behind_them': size(behind),
    'kinds_in_groups': dict(Counter(U[k]['kind'] for k in ing)),
    'classes_in_groups': dict(Counter(str(U[k]['class']) for k in ing)),
    'frozen_in_groups': sum(1 for k in ing if U[k]['block_movers'] == 0),
    'T1_necessary_at_state': sum(1 for u in U.values() if u.get('T1_necessary')),
    'landable_and_T1_necessary_at_state': sum(1 for k in landed if U[k].get('T1_necessary')),
    'left_not_T1_necessary_at_state': sum(1 for k in L if not U[k].get('T1_necessary'))}))
