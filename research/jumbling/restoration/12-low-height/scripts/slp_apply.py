# Apply a SAFE-ALL straight-line program to exact labelled lattice poses, without
# expanding powers. Research simulator evidence only; no native/runtime imports.
# From the repository root:
#   python3 -I -B research/jumbling/restoration/12-low-height/scripts/slp_apply.py \
#       (record|table) <state> <program json> <out table json> [--unit <unit json>]
# Program: {"segments": [word/word_file/core_file/slp segments, chronological]}.
# Segment and letter indices in errors and JSON lines are zero-based.
#
# Soundness: every reachable leaf must belong to buffer.Alphabet rebuilt at the
# current state. A SAFE-ALL letter is admissible, contains no off piece, moves no
# off piece and keeps every lattice piece on the lattice. Thus off geometry and
# the SAFE-ALL alphabet persist after EVERY prefix, including inside a power.
# Only lattice pieces are changed by an SLP. For a chamber x and pose k, a leaf
# (c,g) sends (x,k) to (g.x,g.k) inside its cap, and fixes it outside. Store both
# pi(x) and r(x): u then v has pi_v[pi_u], r_v[pi_u] * r_u. The inverse at y has
# pi_u^-1(y) and r_u[pi_u^-1(y)]^-1. Chamber-fixed rotations must be retained.
# On a cycle based at x_0, Q(m) is the rotation after m steps and H=Q(L).
# Q(qL+s)=Q(s)*H^q; from x_j, r_{u^e}=Q(j+e)*Q(j)^-1. Holonomy powers reduce
# modulo their finite group orders. Exponents and expanded lengths are Python
# integers throughout; neither powers nor inverse SLPs are expanded to words.
# Literal words/core P.Q.P^-1 instead use the simulator, checking every twist.
# The output is a current-only buffer.export_pose_table, not an expanded journal.
import argparse
import gzip
import json
import math
import os
import sys
from pathlib import Path
from types import SimpleNamespace

# Keep NumPy's dependencies single-threaded, including in isolated CLI mode.
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_name] = '1'
import numpy as np

sys.dont_write_bytecode = True
if hasattr(sys, 'set_int_max_str_digits'):
    sys.set_int_max_str_digits(0)
HERE = Path(__file__).resolve().parent


def modules():
    sys.path[:0] = [str(HERE), str(HERE.parents[2] / 'theory')]
    import buffer
    import screen
    return buffer, screen


def require(value, reason):
    if not value:
        raise ValueError(reason)


def read_json(path):
    path = str(path)
    opener = gzip.open if path.endswith('.gz') else open
    with opener(path, 'rt', encoding='utf-8') as stream:
        return json.load(stream)


def print_row(row):
    print(json.dumps(row), flush=True)


class Rotations:
    """Exact K+ products, inverses and finite powers (about 100 MiB for K+)."""
    def __init__(self, kp):
        size = len(kp.perms)
        require(size <= 65536, 'rotation indices do not fit uint16')
        self.mul = np.empty((size, size), np.uint16)
        indices = np.arange(size)
        for k in range(size):
            self.mul[k] = kp.compose_vec(np.full(size, k), indices)
        require(np.array_equal(self.mul[0], indices) and np.array_equal(self.mul[:, 0], indices),
                'K+ identity is not index zero')
        self.inv = np.asarray(kp.inv, np.uint16)
        self.orders = np.zeros(size, np.int32)
        current = np.zeros(size, np.uint16)
        powers = [current]
        for exponent in range(1, size + 1):
            current = self.mul[indices, current]
            powers.append(current)
            self.orders[(self.orders == 0) & (current == 0)] = exponent
            if self.orders.all():
                break
        require(self.orders.all(), 'K+ element order exceeds group order')
        self.powers = np.asarray(powers, np.uint16)
        self.exponent = math.lcm(*np.unique(self.orders).tolist())

    def power_map(self, exponent):
        # Reduce the Python integer before giving indices to NumPy.
        residues = np.zeros(int(self.orders.max()) + 1, np.int32)
        for order in np.unique(self.orders).tolist():
            residues[order] = exponent % order
        return self.powers[residues[self.orders], np.arange(len(self.orders))]


def program(G, cert, root):
    """Check DAG/lengths globally, SAFE membership for every reachable leaf."""
    require(isinstance(cert, dict) and cert.get('format') == 'r22b10/1', 'certificate format must be r22b10/1')
    nodes = cert.get('nodes')
    require(isinstance(nodes, list) and nodes, 'program is empty or malformed')
    use_v = root == 'V'
    if use_v:
        root = cert.get('V')
    require(type(root) is int and 0 <= root < len(nodes), 'node ID out of range')
    children, lengths = [], []
    for ident, node in enumerate(nodes):
        require(isinstance(node, dict), f'node {ident}: malformed node')
        op = node.get('op')
        if op == 'letter':
            require(type(node.get('grip')) is int and type(node.get('item')) is int,
                    f'node {ident}: malformed letter')
            refs, length = [], 1
        elif op == 'cat':
            refs = node.get('args')
            require(isinstance(refs, list), f'node {ident}: cat args must be a list')
        elif op in ('inv', 'pow'):
            refs = [node.get('arg')]
            if op == 'pow':
                require(type(node.get('exp')) is int and node['exp'] >= 0,
                        f'node {ident}: power exponent must be a nonnegative integer')
        else:
            raise ValueError(f'node {ident}: unknown program operation')
        require(all(type(n) is int and 0 <= n < ident for n in refs),
                f'node {ident}: cyclic, forward or out-of-range reference')
        if op == 'cat':
            length = sum(lengths[n] for n in refs)
        elif op in ('inv', 'pow'):
            length = lengths[refs[0]] * (node['exp'] if op == 'pow' else 1)
        if 'length' in node:
            require(type(node['length']) is int and node['length'] == length,
                    f'node {ident}: false expanded length')
        children.append(refs)
        lengths.append(length)
    if use_v and 'V_length' in cert:
        require(type(cert['V_length']) is int and cert['V_length'] == lengths[root], 'false V_length')
    needed, leaves, todo = set(), {}, [root]
    while todo:
        ident = todo.pop()
        if ident in needed:
            continue
        needed.add(ident)
        if nodes[ident]['op'] == 'letter':
            pair = (nodes[ident]['grip'], nodes[ident]['item'])
            require(pair in G.index, f'node {ident}: {pair} is not a SAFE-ALL letter at this state')
            leaves[ident] = G.index[pair]
        todo.extend(children[ident])
    return root, children, lengths, needed, leaves


class Evaluator:
    """All chambers, chronological products, and live-reference reclamation.

    Leaves (and inverses of leaves) remain compact alphabet indices until used;
    every other live value is a source map and a K+ rotation array. No memo keeps
    already consumed nodes, and no recursive call depends on the DAG depth.
    """
    def __init__(self, G, rotations, cert, root='V'):
        self.G, self.rotations, self.nodes = G, rotations, cert['nodes']
        self.root, self.children, self.lengths, self.needed, self.leaves = program(G, cert, root)
        self.indices = np.arange(G.NP, dtype=np.int32)

    def identity(self):
        return self.indices.copy(), np.zeros(self.G.NP, np.uint16)

    def materialize(self, value):
        if not isinstance(value, int):
            return value
        c, _, k, images = self.G.letters[value]
        perm, rotation = self.identity()
        perm[self.G.capx[c]] = images
        rotation[self.G.capx[c]] = k
        return perm, rotation

    def compose(self, first, second):
        p, r = first
        q, s = second
        return q[p], self.rotations.mul[s[p], r]

    def inverse(self, value):
        p, r = value
        back = np.empty_like(p)
        back[p] = self.indices
        return back, self.rotations.inv[r[back]]

    def powered(self, value, exponent):
        require(type(exponent) is int and exponent >= 0, 'power exponent must be a nonnegative integer')
        if exponent == 0:
            return self.identity()
        if exponent == 1:
            return value
        p, r = value
        mul, inv = self.rotations.mul, self.rotations.inv
        out_p = self.indices.copy()
        # Fixed chambers can have nontrivial holonomy: power their rotations too.
        out_r = self.rotations.power_map(exponent)[r]
        seen = p == self.indices
        groups = {}
        for start in np.flatnonzero(~seen).tolist():
            if seen[start]:
                continue
            cycle, prefix, x, rotation = [], [], start, 0
            while not seen[x]:
                seen[x] = True
                cycle.append(x)
                prefix.append(rotation)
                rotation = int(mul[int(r[x]), rotation])
                x = int(p[x])
            require(x == start, 'node action is not a permutation')
            groups.setdefault(len(cycle), []).append((cycle, prefix, rotation))
        for size, rows in groups.items():
            xs = np.asarray([row[0] for row in rows], np.int32)
            prefix = np.asarray([row[1] for row in rows], np.uint16)
            holonomy = np.asarray([row[2] for row in rows], np.uint16)
            quotient, shift = divmod(exponent, size)
            positions = np.arange(size) + shift
            slots, wraps = positions % size, positions >= size
            hq = self.rotations.power_map(quotient)[holonomy]
            hq1 = mul[holonomy, hq]
            tail = np.where(wraps[None, :], hq1[:, None], hq[:, None])
            out_p[xs] = xs[:, slots]
            out_r[xs] = mul[mul[prefix[:, slots], tail], inv[prefix]]
        return out_p, out_r

    def evaluate(self):
        uses = {}
        for ident in self.needed:
            for child in self.children[ident]:
                uses[child] = uses.get(child, 0) + 1
        values = {}
        for ident in sorted(self.needed):
            node, refs = self.nodes[ident], self.children[ident]
            op = node['op']
            if op == 'letter':
                result = int(self.leaves[ident])
            elif op == 'cat':
                result = self.materialize(values[refs[0]]) if refs else self.identity()
                for child in refs[1:]:
                    result = self.compose(result, self.materialize(values[child]))
            elif op == 'inv':
                child = values[refs[0]]
                result = int(self.G.inv[child]) if isinstance(child, int) else self.inverse(child)
            else:
                result = self.powered(self.materialize(values[refs[0]]), node['exp'])
            values[ident] = result
            for child in refs:
                uses[child] -= 1
                if uses[child] == 0:
                    del values[child]
        return self.materialize(values[self.root])


def apply_labelled(st, G, rotations, value):
    """Change lattice poses only, checking their chamber projection and off poses."""
    B, _ = modules()
    require(B.off_fingerprint(st) == G.fingerprint, 'SAFE-ALL geometry changed since alphabet construction')
    kp, reg = st.ctx.kplus, st.ctx.regions
    K = st._kpose()[st.pose_id]
    lattice = np.flatnonzero(K >= 0)
    chambers = reg.piece_of[reg.orbit_of[lattice], kp.compose_vec(K[lattice], reg.transport[lattice])]
    require(len(np.unique(chambers)) == len(chambers), 'two lattice pieces occupy the same chamber')
    p, r = value
    require(p.shape == r.shape == (len(st.pose_id),), 'wrong labelled action size')
    new_k = rotations.mul[r[chambers], K[lattice]]
    destination = reg.piece_of[reg.orbit_of[lattice], kp.compose_vec(new_k, reg.transport[lattice])]
    require(np.array_equal(destination, p[chambers]), 'rotations disagree with the chamber permutation')
    unique, inverse = np.unique(new_k, return_inverse=True)
    ids = np.asarray([st._intern_k(int(k)) for k in unique], np.int32)
    st.pose_id[lattice] = ids[inverse]
    st._compact()
    st.journal, st._anchor_cache = [], {}
    require(B.off_fingerprint(st) == G.fingerprint, 'SLP changed an off piece')
    B.hole_set(st)  # also checks collision freedom and vacancy cardinality


def word_pairs(word, M):
    require(isinstance(word, list), 'word must be a list of integer (grip, item) pairs')
    for j, pair in enumerate(word):
        require(isinstance(pair, (list, tuple)) and len(pair) == 2 and all(type(x) is int for x in pair),
                f'letter {j}: malformed (grip, item) pair')
        require(0 <= pair[0] < 600 and 0 <= pair[1] < len(M.items), f'letter {j}: grip or item out of range')
    return word


def inverse_word(M, word):
    B, _ = modules()
    lookup = {}
    for j, (_, matrix) in enumerate(M.items):
        lookup.setdefault(B.restore.to_tuple(matrix), j)
    out = []
    for c, i in reversed(word):
        key = B.restore.to_tuple(B.restore.transpose(M.items[i][1]))
        require(key in lookup, f'letter ({c}, {i}) has no inverse in the menu')
        out.append([c, lookup[key]])
    return out


def apply_segment(st, M, segment, index=0, rotations=None):
    """Return a segment's JSON row and the reusable state-independent K+ table."""
    B, _ = modules()
    try:
        require(isinstance(segment, dict), 'segment must be an object')
        kinds = [key for key in ('word', 'word_file', 'core_file', 'slp') if key in segment]
        require(len(kinds) == 1, 'segment must have exactly one kind')
        kind = kinds[0]
        if kind == 'slp':
            if 'state_digest' in segment:
                require(segment['state_digest'] == st.digest(), 'state_digest mismatch')
            inverse = segment.get('inverse', False)
            require(type(inverse) is bool, 'inverse must be a boolean')
            cert = read_json(segment['slp'])
            G = B.Alphabet(SimpleNamespace(st=st), M)
            # Validate every reachable leaf before evaluating or changing poses.
            root = segment.get('node', 'V')
            program(G, cert, root)
            if rotations is None:
                rotations = Rotations(st.ctx.kplus)
            evaluator = Evaluator(G, rotations, cert, root)
            value = evaluator.evaluate()
            if inverse:
                value = evaluator.inverse(value)
            length = str(evaluator.lengths[evaluator.root])
            apply_labelled(st, G, rotations, value)
        else:
            if kind == 'word':
                word = segment['word']
            elif kind == 'word_file':
                word = read_json(segment['word_file'])['word']
            else:
                core = read_json(segment['core_file'])
                P, Q = word_pairs(core['P'], M), word_pairs(core['Q'], M)
                word = P + Q + inverse_word(M, P)
            word = word_pairs(word, M)
            for j, (c, i) in enumerate(word):
                twist = B.sim.Twist(st.ctx, c, B.restore.element(M, c, i), 'menu', {'menu': M.name})
                outcome = st.apply(twist, journal=False)
                require(outcome.applied and not outcome.uncertain,
                        f'letter {j} ({c}, {i}): twist is not admissible ({outcome.status})')
            length = len(word)
        return {'segment': index, 'kind': kind, 'length': length,
                'digest': st.digest(), 'off': int(st.off_lattice_count())}, rotations
    except (ValueError, AssertionError, KeyError, TypeError, IndexError, OSError, OverflowError) as error:
        raise ValueError(f'segment {index}: {error}') from error


def run_program(st, M, doc, unit=None, emit=print_row):
    B, _ = modules()
    require(isinstance(doc, dict) and isinstance(doc.get('segments'), list), 'program needs a segments list')
    start = st.digest()
    if unit is not None:
        require(isinstance(unit, dict) and isinstance(unit.get('pieces'), list), 'unit needs a pieces list')
        require(all(type(p) is int and 0 <= p < len(st.pose_id) for p in unit['pieces']), 'unit piece out of range')
        U = set(unit['pieces'])
        off0 = set(B.off_pieces(st).tolist())
        pose0 = {p: st._poses[int(st.pose_id[p])].key for p in off0}
    rotations = None
    for index, segment in enumerate(doc['segments']):
        row, rotations = apply_segment(st, M, segment, index, rotations)
        emit(row)
    summary = {'start_digest': start, 'end_digest': st.digest()}
    if unit is not None:
        off1 = set(B.off_pieces(st).tolist())
        others = off0 - U
        kept = sum(p in off1 and st._poses[int(st.pose_id[p])].key == pose0[p] for p in others)
        landed = len(others - off1)
        on_lattice, collateral = len(U - off1), len(off1 - off0)
        summary.update(unit_size=len(U), unit_on_lattice=on_lattice, collateral=collateral,
                       others=len(others), others_exact=kept, others_landed=landed,
                       off_before=len(off0), off_after=len(off1),
                       strict_prime=on_lattice == len(U) and collateral == 0 and kept + landed == len(others))
    emit(summary)
    return summary


def main(argv=None):
    ap = argparse.ArgumentParser(description='Apply an exact SAFE-ALL SLP and literal words to a pose table.')
    ap.add_argument('mode', choices=('record', 'table'))
    ap.add_argument('state')
    ap.add_argument('program')
    ap.add_argument('out_table')
    ap.add_argument('--unit')
    args = ap.parse_args(argv)
    try:
        doc = read_json(args.program)
        unit = read_json(args.unit) if args.unit else None
        B, screen = modules()
        st, M, _ = screen.load(args.mode, args.state)
        run_program(st, M, doc, unit)
        B.export_pose_table(st, args.out_table)
        return 0
    except (ValueError, AssertionError, KeyError, TypeError, IndexError, OSError, OverflowError) as error:
        print_row({'error': str(error)})
        return 1


if __name__ == '__main__':
    sys.exit(main())
