# Independent exact R22B-10 checker; no builder or R22B-01 imports.
# From the repository root:
#   python3 -I -B research/jumbling/restoration/12-low-height/scripts/check_r22b10.py \
#       (record|table) <state file> <target json> <certificate[.gz]>
# API: check(alphabet, target, certificate, state_digest=None) -> summary,
# or ValueError. Supply the actual state's digest for duck alphabets without
# buffer.Alphabet's state_digest attribute.
# Only buffer.py, screen.py, NumPy and the standard library are dependencies.
# Digests, inventories and global Jordan witnesses precede program verification.
# Every SLP leaf belongs to the independently loaded SAFE alphabet. A SAFE-ALL
# orbit is invariant under each leaf, hence under cat, inv and pow. Consequently
# evaluating V on every target orbit gives exactly the same target occupancy as
# evaluating it on all chambers, including collateral effects of earlier steps.
# Integer powers are evaluated by cycles, never by expanding their words.
# Completed edges, not their interior letters, must fix their anchors.
# Incomplete certificates always FAIL; they say nothing about nonexistence.
# r22b10/2 independently reconstructs atlas seeds and checks all of them,
# including their literal leaves and protected domains, before any word table.
import argparse
import gzip
import hashlib
import json
import math
import sys
from collections import OrderedDict
from pathlib import Path
from types import SimpleNamespace

import numpy as np

sys.dont_write_bytecode = True
if hasattr(sys, 'set_int_max_str_digits'):
    sys.set_int_max_str_digits(0)
HERE = Path(__file__).resolve().parent


def buffer_module():
    sys.path[:0] = [str(HERE), str(HERE.parents[2] / 'theory')]
    import buffer
    return buffer


def require(value, reason):
    if not value:
        raise ValueError(reason)


def integer(value, reason, minimum=0):
    require(type(value) is int and value >= minimum, reason)
    return value


def pair_numbers(G, pairs):
    require(isinstance(pairs, list), 'word must be a list of SAFE letter pairs')
    numbers = []
    for pair in pairs:
        require(isinstance(pair, list) and len(pair) == 2 and all(type(v) is int for v in pair),
                'malformed SAFE letter pair')
        require(tuple(pair) in G.index, 'word contains a non-SAFE letter')
        numbers.append(G.index[tuple(pair)])
    return numbers


def program(G, nodes):
    """Validate every node, including unused nodes; independently derive lengths."""
    require(isinstance(nodes, list) and nodes, 'program is empty or malformed')
    lengths, children, leaves = [], [], {}
    for ident, node in enumerate(nodes):
        require(isinstance(node, dict), 'malformed program node')
        op = node.get('op')
        if op == 'letter':
            numbers = pair_numbers(G, [[node.get('grip'), node.get('item')]])
            leaves[ident] = numbers[0]
            refs, length = [], 1
        elif op == 'cat':
            refs = node.get('args')
            require(isinstance(refs, list), 'cat args must be a list')
            length = None
        elif op in ('inv', 'pow'):
            refs = [node.get('arg')]
            length = None
            if op == 'pow':
                integer(node.get('exp'), 'power exponent must be a nonnegative integer')
        else:
            raise ValueError('unknown program operation')
        for n in refs:
            require(type(n) is int and 0 <= n < ident, 'cyclic, forward or out-of-range program reference')
        if op == 'cat':
            length = sum(lengths[n] for n in refs)
        elif op == 'inv':
            length = lengths[refs[0]]
        elif op == 'pow':
            length = lengths[refs[0]] * node['exp']
        if 'length' in node:
            require(type(node['length']) is int and node['length'] == length, 'false expanded length on program node')
        lengths.append(length)
        children.append(refs)
    return lengths, children, leaves


class Evaluator:
    """Independent topological evaluation with live-reference reclamation."""
    def __init__(self, G, members, nodes, children, leaves, cache_bytes=48 * 1024**2):
        self.G, self.members, self.nodes = G, members, nodes
        self.children, self.leaves = children, leaves
        self.identity = np.arange(len(members), dtype=np.int32)
        self.memo = OrderedDict()
        self.max_entries = max(1, cache_bytes // max(1, self.identity.nbytes))

    def remember(self, ident, result):
        self.memo[ident] = result
        self.memo.move_to_end(ident)
        while len(self.memo) > self.max_entries:
            self.memo.popitem(last=False)

    def powered(self, perm, exponent):
        result = self.identity.copy()
        visited = set()
        for start in range(len(perm)):
            if start in visited:
                continue
            cycle, current = [], start
            while current not in visited:
                visited.add(current)
                cycle.append(current)
                current = int(perm[current])
            size = len(cycle)
            offset = exponent % size
            arr = np.array(cycle, np.int32)
            result[arr] = arr[(np.arange(size) + offset) % size]
        return result

    def evaluate(self, root):
        if root in self.memo:
            self.memo.move_to_end(root)
            return self.memo[root]
        needed, boundary, todo = set(), {}, [root]
        while todo:
            ident = todo.pop()
            if ident in self.memo:
                boundary[ident] = self.memo[ident]
            elif ident not in needed:
                needed.add(ident)
                todo.extend(self.children[ident])
        uses = {}
        for ident in needed:
            for child in self.children[ident]:
                uses[child] = uses.get(child, 0) + 1
        values = boundary
        # IDs already give a topological ordering. Cached boundary values are
        # pinned here even when adding a result evicts their memo entries.
        for ident in sorted(needed):
            node, refs = self.nodes[ident], self.children[ident]
            op = node['op']
            if op == 'letter':
                images = self.G.images([self.leaves[ident]], self.members)
                result = np.searchsorted(self.members, images)
                require((result < len(self.members)).all(), 'SAFE word leaves its orbit')
                require(np.array_equal(self.members[result], images), 'SAFE word leaves its orbit')
                result = result.astype(np.int32)
            elif op == 'cat':
                result = self.identity.copy()
                for child in refs:
                    result = values[child][result]
            elif op == 'inv':
                result = np.argsort(values[refs[0]]).astype(np.int32)
            else:
                result = self.powered(values[refs[0]], node['exp'])
            values[ident] = result
            self.remember(ident, result)
            for child in refs:
                uses[child] -= 1
                if uses[child] == 0:
                    del values[child]
        return values[root]


def node_id(value, nodes):
    require(type(value) is int and 0 <= value < len(nodes), 'node ID out of range')
    return value


def atlas_word(G, B, row, D):
    """Reconstruct a placed AST prefix, independently of the builder."""
    ctx, kp = B.CL.ctx, B.CL.ctx.kplus
    atlas = json.loads((HERE.parents[4] / 'assets/seed_atlas.json').read_text())
    kind = integer(row.get('atlas_type'), 'invalid atlas_type')
    require(kind < len(atlas) and kind == int(ctx.data.orbit_id[D[0]]), 'atlas_type differs from orbit type')
    depth = integer(row.get('atlas_depth'), 'invalid atlas_depth')
    t = atlas[kind]['ast']
    for _ in range(depth):
        require(t[0] != 'g', 'atlas_depth exceeds left spine')
        t = t[1]
    sigma = integer(row.get('sigma'), 'invalid atlas sigma')
    require(sigma < len(kp.perms), 'atlas sigma out of range')

    def expand(t):
        if t[0] == 'g':
            return [t[1]]
        left = expand(t[1])
        if t[0] == 'pow':
            return left * t[2]
        require(t[0] == 'comm', 'unknown atlas AST operation')
        right = expand(t[2])
        return left + right + [-x for x in reversed(left)] + [-x for x in reversed(right)]

    word = np.asarray(expand(t), np.int64)
    rotations = kp.gen_idx[np.abs(word) - 1]
    rotations = np.where(word > 0, rotations, kp.inv[rotations])
    rotations = kp.compose_vec(np.full(len(word), sigma),
                               kp.compose_vec(rotations, np.full(len(word), int(kp.inv[sigma]))))
    grips = kp.perms[sigma, (np.abs(word) - 1) // 2]
    lookup = {(int(g), int(k)): n for n, (g, _, k, _) in enumerate(G.letters)}
    require(all(int(g) in G.by_grip for g in grips), 'atlas placement contains a non-SAFE-ALL grip')
    require(all((int(g), int(k)) in lookup for g, k in zip(grips, rotations)),
            'atlas placement contains a non-SAFE letter')
    return [lookup[(int(g), int(k))] for g, k in zip(grips, rotations)]


def atlas_preflight(G, domains, rows, words, nodes, lengths, leaves):
    """Fail on false atlas evidence before entering expensive table checks."""
    protected = np.zeros(G.NP, bool)
    for D, row, nums in zip(domains, rows, words):
        if row['route'] == 'atlas':
            root = node_id(row.get('seed'), nodes)
            require(lengths[root] == len(nums), 'atlas seed node length differs from seed_word')
            # Expand just this bounded seed. Inverse reverses chronology and
            # inverts every leaf; zero-length powers require no repetitions.
            todo, emitted = [(root, False)], 0
            while todo:
                ident, inverse = todo.pop()
                if lengths[ident] == 0:
                    continue
                node = nodes[ident]
                if node['op'] == 'letter':
                    n = int(G.inv[leaves[ident]]) if inverse else leaves[ident]
                    require(emitted < len(nums) and n == nums[emitted],
                            'atlas seed node expansion differs from seed_word')
                    emitted += 1
                elif node['op'] == 'cat':
                    args = node['args'] if inverse else reversed(node['args'])
                    todo.extend((child, inverse) for child in args)
                elif node['op'] == 'inv':
                    todo.append((node['arg'], not inverse))
                else:
                    todo.extend([(node['arg'], inverse)] * node['exp'])
            require(emitted == len(nums), 'atlas seed node expansion differs from seed_word')
            cycle = row.get('cycle')
            require(isinstance(cycle, list) and len(cycle) == 3 and len(set(cycle)) == 3
                    and all(type(x) is int and x in D for x in cycle), 'invalid atlas cycle')
            require(cycle == row.get('anchors'), 'atlas cycle differs from seed anchors')
            p = G.perm(nums)
            want = D.copy()
            for x, y in zip(cycle, cycle[1:] + cycle[:1]):
                want[np.searchsorted(D, x)] = y
            require(cycle[0] == int(D[np.flatnonzero(want != D)[0]]) and np.array_equal(p[D], want),
                    'atlas seed is not the stated oriented cycle')
            previous = np.flatnonzero(protected)
            require(np.array_equal(p[previous], previous), 'atlas seed moves an earlier target orbit')
            require(type(row.get('s')) is int and row['s'] == len(nums), 'atlas seed length s differs')
        # Every independently recomputed earlier domain is protected in full.
        protected[D] = True


def tree(entries, D, fixed, root, forward, evaluator, lengths):
    require(isinstance(entries, list), 'malformed word table')
    members = set(D.tolist()) - set(fixed)
    parents, edge_lengths = {}, {}
    local = {int(x): j for j, x in enumerate(D)}
    anchor_indices = [local[x] for x in fixed]
    for entry in entries:
        require(isinstance(entry, list) and len(entry) == 3 and all(type(x) is int for x in entry),
                'malformed word table entry')
        parent, child, ident = entry if forward else [entry[1], entry[0], entry[2]]
        require(child in members - {root} and parent in members, 'word table vertex or parent out of range')
        require(child not in parents, 'duplicate word table child')
        node_id(ident, evaluator.nodes)
        p = evaluator.evaluate(ident)
        require(np.array_equal(p[anchor_indices], anchor_indices), 'completed edge word moves a fixed anchor')
        src, dst = (parent, child) if forward else (child, parent)
        require(int(p[local[src]]) == local[dst], 'completed edge word has wrong child/parent action')
        parents[child], edge_lengths[child] = parent, lengths[ident]
    require(set(parents) == members - {root}, 'word table does not cover every required chamber exactly once')
    costs = {root: 0}
    for start in parents:
        path, seen, x = [], set(), start
        while x not in costs:
            require(x not in seen, 'parent cycle in word table')
            require(x in parents, 'word table path does not end at its root')
            seen.add(x)
            path.append(x)
            x = parents[x]
        cost = costs[x]
        for x in reversed(path):
            cost += edge_lengths[x]
            costs[x] = cost
    return max(costs.values())


def check(G, target, cert, state_digest=None):
    require(isinstance(target, dict) and isinstance(cert, dict), 'target and certificate must be objects')
    fmt = cert.get('format')
    require(fmt in ('r22b10/1', 'r22b10/2'), 'certificate format')
    actual_digest = getattr(G, 'state_digest', state_digest)
    require(state_digest is None or state_digest == actual_digest, 'state digest differs from alphabet state')
    require(actual_digest is not None and actual_digest == target.get('state_digest') == cert.get('state_digest'), 'state digest mismatch')
    T = target.get('T')
    require(isinstance(T, list) and all(type(x) is int for x in T), 'target chambers must be integers')
    digest = hashlib.sha256(json.dumps(sorted(T)).encode()).hexdigest()
    require(digest == target.get('T_digest') == cert.get('T_digest'), 'target digest mismatch')
    require(len(set(T)) == len(T) and all(0 <= x < G.NP for x in T), 'target chambers duplicated or out of range')
    require(type(cert.get('T_size')) is int and cert['T_size'] == len(T), 'T_size mismatch')
    orbit, sizes = G.orbits()
    B = buffer_module()
    condition = B.condition(G, T)
    require(condition['fixed_vacant'], 'N1 fails: occupied singleton target')
    require(condition['necessary'], 'N2 fails: orbit inventory')
    require(cert.get('complete') is True, 'incomplete certificate: ' + json.dumps(cert.get('missing', [])))
    require(cert.get('missing') == [], 'complete certificate has missing orbits')
    require(type(cert.get('seed')) is int, 'seed must be an integer')
    require(type(cert.get('seconds')) in (int, float) and math.isfinite(cert['seconds']) and cert['seconds'] >= 0,
            'invalid elapsed seconds')
    labels = sorted({int(orbit[x]) for x in T if sizes[orbit[x]] > 1},
                    key=lambda o: (-int(sizes[o]), int(np.flatnonzero(orbit == o)[0])))
    domains = [np.flatnonzero(orbit == o) for o in labels]
    if fmt == 'r22b10/2':
        require(cert.get('seeds') == 'atlas', 'r22b10/2 seeds must be atlas')
        order = cert.get('orbit_order')
        by_least = {int(D[0]): (lab, D) for lab, D in zip(labels, domains)}
        require(isinstance(order, list) and all(type(x) is int for x in order)
                and len(order) == len(by_least) and set(order) == set(by_least),
                'orbit_order must contain every nontrivial target orbit exactly once')
        labels = [by_least[x][0] for x in order]
        domains = [by_least[x][1] for x in order]
    rows = cert.get('orbits')
    require(isinstance(rows, list) and len(rows) == len(labels), 'target orbit list differs')
    # Validate global Jordan words and reconstruct atlas words before trusting
    # any program nodes. No failed row can shrink the protected domain union.
    atlas_words = []
    for lab, D, row in zip(labels, domains, rows):
        require(isinstance(row, dict), 'malformed orbit row')
        require(all(type(row.get(k)) is int for k in ('least', 'size', 'k')), 'malformed orbit sizes')
        require((row['least'], row['size'], row['k']) == (int(D[0]), len(D), sum(orbit[x] == lab for x in T)),
                'orbit order, least chamber, size or k differs')
        require(len(D) >= 5, 'target orbit degree below five')
        routes = ('commutator', 'jordan', 'atlas') if fmt == 'r22b10/2' else ('commutator', 'jordan')
        require(row.get('route') in routes, 'unknown seed route')
        nums = None
        if row['route'] == 'atlas':
            declared = pair_numbers(G, row.get('seed_word'))
            nums = atlas_word(G, B, row, D)
            require(declared == nums, 'atlas seed_word differs from recomputed atlas placement')
        if row['route'] == 'jordan':
            nums = pair_numbers(G, row.get('jordan_word'))
            prime = integer(row.get('p'), 'Jordan prime must be an integer', 2)
            witness = B.check_jordan(G, row['jordan_word'], lab, prime)
            _, lengths = B.cycles(G.perm(nums))
            m = 1
            for rep in np.flatnonzero(lengths).tolist():
                if rep != witness['cycle_least_point']:
                    m = math.lcm(m, int(lengths[rep]))
            require(type(row.get('M')) is int and row['M'] == m, 'Jordan M differs from exact global lcm')
        atlas_words.append(nums)
    nodes = cert.get('nodes')
    lengths, children, leaves = program(G, nodes)
    V = node_id(cert.get('V'), nodes)
    require(type(cert.get('V_length')) is int and cert['V_length'] == lengths[V], 'V_length differs from expanded length')
    if fmt == 'r22b10/2':
        atlas_preflight(G, domains, rows, atlas_words, nodes, lengths, leaves)
        last = -1
        order = {int(D[0]): j for j, D in enumerate(domains)}
        require(isinstance(cert.get('steps'), list), 'malformed steps')
        for step in cert['steps']:
            require(isinstance(step, dict) and type(step.get('orbit')) is int and step['orbit'] in order,
                    'step names an unknown target orbit')
            j = order[step['orbit']]
            require(j >= last, 'steps are not in orbit_order')
            last = j
    bound = 0
    summaries = []
    for j, (D, row) in enumerate(zip(domains, rows)):
        ev = Evaluator(G, D, nodes, children, leaves)
        local = {int(x): i for i, x in enumerate(D)}
        anchors = row.get('anchors')
        require(isinstance(anchors, list) and len(anchors) == 3 and all(type(x) is int and x in local for x in anchors)
                and len(set(anchors)) == 3, 'invalid seed anchors')
        a, b, c = anchors
        ident = node_id(row.get('seed'), nodes)
        p = ev.evaluate(ident)
        moved = np.flatnonzero(p != ev.identity)
        require(len(moved) == 3 and a == int(D[moved[0]]), 'seed does not move exactly three points with least anchor a')
        want = ev.identity.copy()
        want[[local[a], local[b], local[c]]] = [local[b], local[c], local[a]]
        require(np.array_equal(p, want), 'seed is not the stated three-cycle (a b c)')
        for prior in domains[:j]:
            before = Evaluator(G, prior, nodes, children, leaves, cache_bytes=4 * 1024**2)
            require(np.array_equal(before.evaluate(ident), before.identity), 'seed moves an earlier target orbit')
        s = lengths[ident]
        require(type(row.get('s')) is int and row['s'] == s, 'seed expanded length s differs')
        if row['route'] == 'jordan':
            h = node_id(row.get('h'), nodes)
            r = int(np.count_nonzero(ev.evaluate(h) != ev.identity))
            require(type(row.get('r')) is int and row['r'] == r and 2 <= r and 2 * r - 1 <= len(D),
                    'Jordan h support r does not meet the small-support hypothesis')
            if 'U' in row:
                node_id(row['U'], nodes)
        e = tree(row.get('P'), D, [a], b, False, ev, lengths)
        f = tree(row.get('Q'), D, [a, b], c, True, ev, lengths)
        require(type(row.get('e')) is int and type(row.get('f')) is int and (row['e'], row['f']) == (e, f),
                'table path cost e or f differs')
        bound += 2 * row['k'] * (s + 2 * e + 2 * f)
        summaries.append(dict(least=int(D[0]), size=len(D), k=row['k'], route=row['route'], s=s, e=e, f=f))
    require(type(cert.get('bound')) is int and cert['bound'] == bound, 'bound differs from recomputed exact bound')
    require(lengths[V] <= bound, 'V_length exceeds bound')
    steps = cert.get('steps')
    require(isinstance(steps, list), 'malformed steps')
    by_least = {int(D[0]): j for j, D in enumerate(domains)}
    grouped = [[] for _ in rows]
    last = -1
    for step in steps:
        require(isinstance(step, dict) and type(step.get('orbit')) is int and step['orbit'] in by_least,
                'step names an unknown target orbit')
        j = by_least[step['orbit']]
        require(j >= last, 'steps are not in orbit order')
        last = j
        cycle = step.get('cycle')
        D = domains[j]
        require(isinstance(cycle, list) and len(cycle) == 3
                and all(type(x) is int and x in D for x in cycle) and len(set(cycle)) == 3, 'invalid step cycle')
        node_id(step.get('node'), nodes)
        grouped[j].append(step)
    for j, (D, row) in enumerate(zip(domains, rows)):
        ev = Evaluator(G, D, nodes, children, leaves)
        local = {int(x): i for i, x in enumerate(D)}
        require(len(grouped[j]) <= row['k'], 'more than k vacating steps on an orbit')
        for step in grouped[j]:
            xyz = [local[x] for x in step['cycle']]
            want = ev.identity.copy()
            want[xyz] = xyz[1:] + xyz[:1]
            require(np.array_equal(ev.evaluate(step['node']), want), 'step is not its stated three-cycle (conjugation or Q direction)')
            require(lengths[step['node']] <= 2 * (row['s'] + 2 * row['e'] + 2 * row['f']), 'step exceeds seed/table length bound')
            for prior in domains[:j]:
                earlier = Evaluator(G, prior, nodes, children, leaves, cache_bytes=4 * 1024**2)
                require(np.array_equal(earlier.evaluate(step['node']), earlier.identity), 'step moves an earlier target orbit')
        pV = ev.evaluate(V)
        final = np.empty(len(D), bool)
        final[pV] = G.occupied[D]
        require(not any(final[local[x]] for x in T if x in local), 'V leaves a target occupied')
        replay = ev.identity.copy()
        # Include ALL steps: earlier orbits' steps may act on this orbit.
        for step in steps:
            replay = ev.evaluate(step['node'])[replay]
        require(np.array_equal(replay, pV), 'V action differs from chronological step replay')
    return dict(format=fmt, complete=True, T_size=len(T), orbits=summaries,
                steps=len(steps), V_length=lengths[V], bound=bound)


def main(argv=None):
    ap = argparse.ArgumentParser(description='Independently check an R22B-10 certificate.')
    ap.add_argument('mode', choices=('record', 'table'))
    ap.add_argument('state')
    ap.add_argument('target')
    ap.add_argument('certificate')
    args = ap.parse_args(argv)
    try:
        target = json.loads(Path(args.target).read_text())
        opener = gzip.open if args.certificate.endswith('.gz') else open
        with opener(args.certificate, 'rt', encoding='utf-8') as stream:
            cert = json.load(stream)
        # Reject wrong target evidence before expensive geometry/state loading.
        require(isinstance(target.get('T'), list) and all(type(x) is int for x in target['T']), 'malformed target')
        require(hashlib.sha256(json.dumps(sorted(target['T'])).encode()).hexdigest() == target['T_digest'] == cert['T_digest'],
                'target digest mismatch')
        B = buffer_module()
        import screen
        st, M, _ = screen.load(args.mode, args.state)
        require(st.digest() == target['state_digest'] == cert['state_digest'], 'state digest mismatch')
        G = B.Alphabet(SimpleNamespace(st=st), M)
        summary = check(G, target, cert, state_digest=st.digest())
        print(json.dumps(summary), flush=True)
        print('PASS', flush=True)
        return 0
    except (ValueError, AssertionError, KeyError, TypeError, IndexError, OSError, OverflowError) as error:
        print(f'FAIL: {error}', flush=True)
        return 1


if __name__ == '__main__':
    sys.exit(main())
