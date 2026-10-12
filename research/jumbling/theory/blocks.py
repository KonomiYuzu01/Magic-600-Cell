"""Rigid off-lattice blocks, retained transport and their locked chambers, exactly.

Run from the repository root, for example:
    python research/jumbling/theory/blocks.py --isolated --replay 2 --out <file>
    python research/jumbling/theory/blocks.py --words research/jumbling/theory/results/e-chain/*.json \
        research/jumbling/theory/results/e-chain-104/07-word-277-preparation.json \
        --coupled 4,6 --max-states 30000 --replay 3 --out <file>

Labels of individual off pieces are 0 (outside), 1 (inside), 2 (straddle).
Block status 3 means members on both sides with none straddling; it is reported
as an inconsistency. A transport multiplier k acts on the left of the original
pose. Isolated edges are k -> k h, whose native matrix is k h k^-1; coupled
edges apply that native matrix on the left of every selected multiplier.

Predecessor files beside --out are structured .npy arrays in BFS order, with
fields key, parent_key, native_grip, native_matrix_index. Keys encode tuples
in base 7200 in the reported block order. The root's parent and letter are -1.
Keys exceeding int64 are stored as decimal strings (without pickle). Searches
with at most two blocks use a dense key-to-row array; larger searches use a
sparse dictionary. --max-states limits reached states, including the root;
complete is true only when the BFS queue has been exhausted.

Evidence kind: source and exact synthetic geometry. Timings are timings of
research code, not native or performance evidence. Scratch caches live only
in a system temporary directory.
"""
import argparse
import json
import random
import sys
import tempfile
import time
from collections import deque
from pathlib import Path

sys.dont_write_bytecode = True

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))

import classes as CL  # noqa: E402
import orient as O  # noqa: E402
import restore  # noqa: E402
import split_pairs as SP  # noqa: E402
from exact import matmul  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def rigid_components(off, labels):
    """Connected components of the relation 'no cut separates the two regions'."""
    inside = np.packbits(labels == 1, axis=1)
    outside = np.packbits(labels == 0, axis=1)
    left = np.ones(len(off), bool)
    components = []
    while left.any():
        first = int(np.flatnonzero(left)[0])
        left[first] = False
        queue, component = deque([first]), []
        while queue:
            p = queue.popleft()
            component.append(p)
            separated = ((inside[p] & outside) | (outside[p] & inside)).any(axis=1)
            neighbours = np.flatnonzero(left & ~separated)
            left[neighbours] = False
            queue.extend(neighbours.tolist())
        components.append(np.sort(off[component]))
    return sorted(components, key=lambda c: int(c[0]))


class GroupTables:
    """Exact K multiplication and retained menus; the multiplication cache is temporary."""

    def __init__(self, M, temporary_directory):
        self.M, self.kp = M, CL.ctx.kplus
        kp = self.kp
        self.n = len(kp.perms)
        self.identity = kp.index_of_matrix(restore.I4)
        if self.n != 7200 or self.identity < 0:
            raise AssertionError('expected K of order 7200 with an identity')
        self.mul = np.lib.format.open_memmap(Path(temporary_directory) / 'k-multiplication.npy',
                                             mode='w+', dtype=np.int16, shape=(self.n, self.n))
        ids = np.arange(self.n, dtype=np.int64)
        for k in range(self.n):
            self.mul[k] = kp.compose_vec(np.full(self.n, k, np.int64), ids)
        self.mul.flush()

        # TwistMenu.frame(c) is the retained frame kp.frame_idx[c]. Thus exact
        # membership at grip 0 transports to every grip by frame conjugation.
        base = [kp.index_of_matrix(restore.element(M, 0, i)) for i in range(len(M.items))]
        frame0_inv = int(kp.inv[int(kp.frame_idx[0])])
        frames = self.mul[kp.frame_idx, frame0_inv]
        self.menu_indices = np.full((600, len(M.items)), -1, np.int16)
        for i, h in enumerate(base):
            if h >= 0:
                self.menu_indices[:, i] = self.mul[self.mul[frames, h], kp.inv[frames]]
        self.retained = [row[(row >= 0) & (row != self.identity)] for row in self.menu_indices]
        log(group_tables=self.n, retained_per_grip=sorted({len(r) for r in self.retained}))

    def menu_index(self, grip, matrix_index):
        hits = np.flatnonzero(self.menu_indices[grip] == matrix_index)
        if not len(hits):
            raise AssertionError(f'native letter ({grip}, K[{matrix_index}]) is not in the menu')
        i = int(hits[0])
        if restore.element(self.M, grip, i) != self.kp.matrix(matrix_index):
            raise AssertionError('native matrix does not match its exact menu element')
        return i

    def close(self):
        self.mul._mmap.close()


class TransportTree:
    """A BFS queue and predecessor tree, with dense indexing for one or two blocks."""

    def __init__(self, blocks, group, max_states=None):
        if max_states is not None and max_states < 1:
            raise ValueError('max_states must be positive')
        self.blocks = tuple(blocks)
        self.n, self.identity = group.n, group.identity
        self.space = self.n ** len(self.blocks)
        self.limit = min(self.space, max_states) if max_states is not None else self.space
        self.dense = len(self.blocks) <= 2
        self.row_of = np.full(self.space, -1, np.int32) if self.dense else {}
        self.wide = self.space > np.iinfo(np.int64).max
        key_type = f'U{len(str(self.space)) + 1}' if self.wide else np.int64
        self.dtype = np.dtype([('key', key_type), ('parent_key', key_type),
                               ('native_grip', np.int16), ('native_matrix_index', np.int16)])
        capacity = self.limit if self.dense else min(self.limit, 1024)
        self.records = np.empty(capacity, self.dtype)
        self.root = self.encode([self.identity] * len(self.blocks))
        self.records[0] = (self.root, -1, -1, -1)
        self.row_of[self.root] = 0
        self.count, self.longest, self.farthest = 1, 0, self.root
        self.complete = False

    def encode(self, state):
        key = 0
        for k in state:
            key = key * self.n + int(k)
        return key

    def keys(self, states):
        keys = np.zeros(len(states), dtype=object if self.wide else np.int64)
        for j in range(len(self.blocks)):
            keys = keys * self.n + states[:, j]
        return keys

    def decode(self, key):
        state = [0] * len(self.blocks)
        for j in range(len(state) - 1, -1, -1):
            key, state[j] = divmod(int(key), self.n)
        return tuple(state)

    def add(self, keys, parent, grips, matrices, depth):
        """Add unseen children; return true only if the state cap omits a child."""
        if self.dense:
            new = self.row_of[keys] < 0
        else:
            new = np.array([int(k) not in self.row_of for k in keys], bool)
        indices = np.flatnonzero(new)
        if not len(indices):
            return False
        _, first = np.unique(keys[indices], return_index=True)
        indices = indices[np.sort(first)]
        room = self.limit - self.count
        stopped = len(indices) > room
        indices = indices[:room]
        end = self.count + len(indices)
        if end > len(self.records):
            grown = np.empty(min(self.limit, max(end, 2 * len(self.records))), self.dtype)
            grown[:self.count] = self.records[:self.count]
            self.records = grown
        rows = self.records[self.count:end]
        rows['key'], rows['parent_key'] = keys[indices], parent
        rows['native_grip'], rows['native_matrix_index'] = grips[indices], matrices[indices]
        if self.dense:
            self.row_of[keys[indices]] = np.arange(self.count, end, dtype=np.int32)
        else:
            for row, k in enumerate(keys[indices], self.count):
                self.row_of[int(k)] = row
        if len(indices):
            self.longest, self.farthest = depth, int(keys[indices[-1]])
        self.count = end
        return stopped

    def path(self, key):
        key = int(key)
        row = int(self.row_of[key]) if self.dense else self.row_of.get(key, -1)
        if row < 0:
            raise ValueError('target state was not reached')
        letters = []
        while key != self.root:
            row = self.records[int(self.row_of[key])]
            letters.append((int(row['native_grip']), int(row['native_matrix_index'])))
            key = int(row['parent_key'])
        return list(reversed(letters))

    def save(self, path):
        np.save(path, self.records[:self.count], allow_pickle=False)

    def summary(self):
        distinct = np.zeros((len(self.blocks), self.n), bool)
        solo = np.zeros(len(self.blocks), np.int64)
        # Bound scratch memory even for the complete 51,840,000-state tree.
        for start in range(0, self.count, 262144):
            raw = self.records['key'][start:min(self.count, start + 262144)]
            keys = np.array([int(k) for k in raw], dtype=object) if self.wide else raw.copy()
            states = np.empty((len(keys), len(self.blocks)), np.int16)
            for j in range(len(self.blocks) - 1, -1, -1):
                states[:, j] = keys % self.n
                keys = keys // self.n
                distinct[j, states[:, j]] = True
            moved = states != self.identity
            alone = moved.sum(axis=1) == 1
            solo += (moved & alone[:, None]).sum(axis=0)
        return {'blocks': list(self.blocks), 'states': self.count, 'complete': self.complete,
                'longest_shortest_word': self.longest, 'root_key': self.root,
                'farthest_key': self.farthest, 'state_space': self.space,
                'distinct_multipliers': {str(b): int(distinct[j].sum()) for j, b in enumerate(self.blocks)},
                'states_with_only_this_block_moved': {str(b): int(solo[j]) for j, b in enumerate(self.blocks)}}


class Blocks:
    """Cut labels, rigid blocks and chambers at one exact reference configuration."""

    def __init__(self, R, M):
        self.R, self.M, self.st = R, M, R.st
        st = self.st
        self.digest = st.digest()
        self.off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
        self.original = {int(p): st.pose(int(p)) for p in self.off}
        self.labels = np.zeros((len(self.off), 600), np.uint8)
        for c in (range(600) if len(self.off) else ()):
            cl = st.classify(c, need_inside=True, stop_at_straddle=False)
            if cl.uncertain:
                raise AssertionError(f'uncertain classification at grip {c}')
            inside = np.isin(self.off, cl.inside, assume_unique=True)
            self.labels[inside, c] = 1
            if cl.straddle is not None:
                for row in np.flatnonzero(~inside):
                    p = int(self.off[row])
                    side = SP.side(st.pose(p), p, c)
                    if side == 'straddle':
                        self.labels[row, c] = 2
                    elif side != 'out':
                        raise AssertionError(f'piece {p} not inside grip {c} has side {side}')
        self.blocks = rigid_components(self.off, self.labels)
        self.status = np.zeros((len(self.blocks), 600), np.uint8)
        for b, pieces in enumerate(self.blocks):
            labels = self.labels[np.searchsorted(self.off, pieces)]
            inside, outside = (labels == 1).all(axis=0), (labels == 0).all(axis=0)
            straddle = (labels == 2).any(axis=0)
            self.status[b] = np.where(straddle, 2, np.where(inside, 1, np.where(outside, 0, 3)))
        self.I = [np.flatnonzero(s == 1) for s in self.status]
        self.S = [np.flatnonzero(s == 2) for s in self.status]
        self.straddling_grips = np.flatnonzero((self.labels == 2).any(axis=0))
        self.locked, self.vacant_locked = self._locked() if self.blocks else ([], [])
        self._classes = None
        log(off=len(self.off), blocks=[len(b) for b in self.blocks], straddling=len(self.straddling_grips),
            status_3=int((self.status == 3).sum()))

    def _locked(self):
        reg, kp = CL.ctx.regions, CL.ctx.kplus
        NP = len(self.st.pose_id)
        K = self.st._kpose()[self.st.pose_id]
        lattice = np.flatnonzero(K >= 0)
        occupied = np.zeros(NP, bool)
        chambers = reg.piece_of[reg.orbit_of[lattice], kp.compose_vec(K[lattice], reg.transport[lattice])]
        if len(np.unique(chambers)) != len(chambers):
            raise AssertionError('two lattice pieces occupy the same chamber')
        occupied[chambers] = True
        # data.sig_matrix supplies the home cap signatures. Pack them in chunks
        # so chamber subset tests do not need a full NP x 600 scratch matrix.
        home = np.empty((NP, 75), np.uint8)
        for start in range(0, NP, 8192):
            end = min(NP, start + 8192)
            home[start:end] = np.packbits(CL.data.sig_matrix(np.arange(start, end)), axis=1)
        locked, vacant = [], []
        for status in self.status:
            required = np.packbits(status == 1)
            allowed = np.packbits((status == 1) | (status == 2))
            ok = ((home & required) == required).all(axis=1) & ((home & ~allowed) == 0).all(axis=1)
            locked.append(np.flatnonzero(ok).tolist())
            vacant.append(np.flatnonzero(ok & ~occupied).tolist())
        return locked, vacant

    def report(self):
        moving = (self.status <= 1).all(axis=0) & (self.status == 1).any(axis=0)
        return {'menu': self.M.name, 'source_digest': self.digest, 'off_lattice': len(self.off),
                'model_identity': dict(CL.data.identity), 'evidence': 'source and exact synthetic geometry',
                'piece_labels': {'pieces': self.off.tolist(), 'labels': self.labels.tolist(),
                                 'legend': {'0': 'outside', '1': 'inside', '2': 'straddle'}},
                'blocks': [{'block': b, 'size': len(pieces), 'pieces': pieces.tolist(),
                            'I_B': self.I[b].tolist(), 'S_B': self.S[b].tolist(),
                            'locked_chambers': self.locked[b], 'vacant_locked_chambers': self.vacant_locked[b]}
                           for b, pieces in enumerate(self.blocks)],
                'status_matrix': self.status.tolist(),
                'status_3': [{'block': int(b), 'grip': int(c)} for b, c in np.argwhere(self.status == 3)],
                'straddling_grips': self.straddling_grips.tolist(),
                'admissible_block_moving_grips': [{'grip': int(c),
                                                  'inside_blocks': np.flatnonzero(self.status[:, c] == 1).tolist()}
                                                 for c in np.flatnonzero(moving)]}

    def _selection(self, blocks):
        selection = tuple(sorted(int(b) for b in blocks))
        if not selection or len(set(selection)) != len(selection):
            raise ValueError('select distinct blocks, at least one')
        if selection[0] < 0 or selection[-1] >= len(self.blocks):
            raise ValueError('block number out of range')
        return selection

    def _generators(self, b, group):
        c, h = [], []
        for grip in self.I[b]:
            retained = group.retained[int(grip)]
            c.extend([int(grip)] * len(retained))
            h.extend(retained.tolist())
        return np.array(c, np.int16), np.array(h, np.int16)

    def _search(self, selection, group, successors, max_states=None):
        tree = TransportTree(selection, group, max_states)
        head, depth, level_end = 0, 0, 1
        stopped = False
        while head < tree.count:
            if head == level_end:
                depth += 1
                level_end = tree.count
            parent = int(tree.records[head]['key'])
            states, grips, matrices = successors(tree.decode(parent))
            if tree.add(tree.keys(states), parent, grips, matrices, depth + 1):
                stopped = True
                break
            head += 1
            if head % 100000 == 0:
                log(transport=list(selection), expanded=head, reached=tree.count)
        tree.complete = not stopped
        log(transport=list(selection), states=tree.count, complete=tree.complete, longest=tree.longest)
        return tree

    def isolated(self, block, group):
        """Other off pieces fixed at every prefix; edges multiply k h on the right."""
        selection = self._selection((block,))
        b = selection[0]
        other = np.arange(len(self.blocks)) != b
        allowed = (self.status[other] == 0).all(axis=0)
        c, h = self._generators(b, group)
        kp, mul = group.kp, group.mul

        def successors(state):
            k = state[0]
            g = kp.perms[k, c]
            keep = allowed[g]
            g, hs = g[keep], h[keep]
            ends = mul[k, hs]
            matrices = mul[ends, int(kp.inv[k])]
            return ends[:, None], g, matrices

        return self._search(selection, group, successors)

    def coupled(self, blocks, group, max_states=None):
        """Move any selected blocks together; every unselected block remains fixed."""
        selection = self._selection(blocks)
        other = ~np.isin(np.arange(len(self.blocks)), selection)
        allowed = (self.status[other] == 0).all(axis=0)
        generators = [self._generators(b, group) for b in selection]
        kp, mul = group.kp, group.mul

        def successors(state):
            grips, matrices = [], []
            for k, (c, h) in zip(state, generators):
                g = kp.perms[k, c]
                keep = allowed[g]
                grips.append(g[keep])
                matrices.append(mul[mul[k, h[keep]], int(kp.inv[k])])
            g, m = np.concatenate(grips), np.concatenate(matrices)
            # A native letter generated by two selected blocks is still one letter.
            _, first = np.unique(g.astype(np.int64) * group.n + m, return_index=True)
            first.sort()
            g, m = g[first], m[first]
            status = np.array([self.status[b, kp.perms[int(kp.inv[k]), g]]
                               for b, k in zip(selection, state)], np.uint8)
            keep = (status <= 1).all(axis=0) & (status == 1).any(axis=0)
            g, m, status = g[keep], m[keep], status[:, keep]
            ends = np.tile(np.asarray(state, np.int16), (len(g), 1))
            for j, k in enumerate(state):
                moved = status[j] == 1
                ends[moved, j] = mul[m[moved], k]
            return ends, g, m

        return self._search(selection, group, successors, max_states)

    def _check_frames(self):
        if self._classes is None:
            self._classes = CL.classes()
        members, cls = self._classes
        D = self.R.domains()
        for d in np.unique(np.asarray(D.dom)[self.off]).tolist():
            if d == 0 or len(CL.hub(D.reps[d], members, cls)) != 1:
                raise AssertionError(f'off-lattice domain {d} is not a frame')

    def replay(self, tree, group, n, seed=7):
        """Replay the farthest state, then seeded random states, undoing each word."""
        if n < 0:
            raise ValueError('replay count must be nonnegative')
        if self.st.digest() != self.digest:
            raise AssertionError('transport reference state has changed')
        if not n:
            return []
        rng = random.Random(seed)
        targets = [tree.farthest] + [int(tree.records[rng.randrange(tree.count)]['key']) for _ in range(n - 1)]
        selected = set(tree.blocks)
        other_blocks = [b for b in range(len(self.blocks)) if b not in selected]
        kp, mul = group.kp, group.mul
        reports = []
        for target in targets:
            word = tree.path(target)
            state = [group.identity] * len(tree.blocks)
            done = 0
            try:
                self._check_frames()
                for g, m in word:
                    if m == group.identity or kp.perms[m, g] != g:
                        raise AssertionError('native letter is identity or does not fix its grip')
                    if any(self.status[b, g] != 0 for b in other_blocks):
                        raise AssertionError('native letter meets an unselected block')
                    moving = []
                    for j, (b, k) in enumerate(zip(tree.blocks, state)):
                        status = int(self.status[b, int(kp.perms[int(kp.inv[k]), g])])
                        if status not in (0, 1):
                            raise AssertionError('native letter cuts a selected block')
                        if status == 1:
                            moving.append((j, b))
                    if not moving:
                        raise AssertionError('native letter moves no selected block')
                    expected = np.sort(np.concatenate([self.blocks[b] for _, b in moving]))
                    cl = self.st.classify(g, need_inside=True, stop_at_straddle=False)
                    if cl.straddle is not None or cl.uncertain:
                        raise AssertionError('transport letter is not physically admissible')
                    actual = np.intersect1d(self.off, cl.inside, assume_unique=True)
                    if not np.array_equal(actual, expected):
                        raise AssertionError('physical inside off pieces disagree with selected blocks')
                    unchanged = np.setdiff1d(self.off, expected, assume_unique=True)
                    before = {int(p): self.st.pose(int(p)) for p in unchanged}
                    self.R.apply(g, group.menu_index(g, m))
                    done += 1
                    for j, _ in moving:
                        state[j] = int(mul[m, state[j]])
                    if not np.array_equal(np.flatnonzero(~self.st.lattice_flags()), self.off):
                        raise AssertionError('retained transport changed the off-lattice set')
                    if any(self.st.pose(p) != pose for p, pose in before.items()):
                        raise AssertionError('a nonselected off pose changed at a prefix')
                    self._check_frames()
                if tuple(state) != tree.decode(target):
                    raise AssertionError('native word does not reach its transport multiplier tuple')
                expected_poses = {}
                for b, k in zip(tree.blocks, state):
                    for p in self.blocks[b].tolist():
                        key = (k, self.original[p])
                        if key not in expected_poses:
                            expected_poses[key] = to_tuple(matmul(kp.matrix(k), self.original[p]))
                        if self.st.pose(p) != expected_poses[key]:
                            raise AssertionError(f'piece {p} has the wrong final exact pose')
            finally:
                for _ in range(done):
                    self.R.undo()
                if self.st.digest() != self.digest:
                    raise AssertionError('physical replay did not undo to the start digest')
            reports.append({'key': target, 'multipliers': list(tree.decode(target)), 'length': len(word),
                            'prefix_checks': len(word), 'passed': True, 'undo_digest': self.digest})
            log(replay_blocks=list(tree.blocks), key=target, length=len(word), passed=True)
        return reports


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--table', help='current-only pose table (default: E2 end)')
    ap.add_argument('--words', nargs='*', default=[], help='certified word files, replayed in supplied order')
    ap.add_argument('--isolated', action='store_true', help='isolated transport of every block')
    ap.add_argument('--coupled', help='comma-separated block numbers, e.g. 4,6')
    ap.add_argument('--max-states', type=int, help='maximum reached coupled states, including the root')
    ap.add_argument('--replay', type=int, default=0, help='physical replays per tree: farthest, then random')
    ap.add_argument('--seed', type=int, default=7, help='seed for random replay targets')
    ap.add_argument('--out', required=True, help='JSON report; predecessor .npy files go beside it')
    a = ap.parse_args(argv)
    if a.replay < 0 or (a.max_states is not None and a.max_states < 1):
        ap.error('--replay must be nonnegative and --max-states must be positive')
    if a.replay and not (a.isolated or a.coupled):
        ap.error('--replay needs --isolated or --coupled')
    if a.max_states is not None and not a.coupled:
        ap.error('--max-states needs --coupled')
    try:
        selection = tuple(int(b) for b in a.coupled.split(',')) if a.coupled is not None else None
    except ValueError:
        ap.error('--coupled must contain comma-separated integer block numbers')
    t0 = time.time()
    output = Path(a.out)
    with tempfile.TemporaryDirectory(prefix='blocks-') as temporary:
        old_cache = O.seams.CACHE
        O.seams.CACHE = Path(temporary) / 'home_centroids.npy'
        group = None
        try:
            R, M = O.start(a.table, a.words)
            model = Blocks(R, M)
            if selection is not None:
                try:
                    selection = model._selection(selection)
                except ValueError as exc:
                    ap.error(str(exc))
            doc = model.report()
            doc.update(start='E2 end of I_a@40' if a.table is None else 'current-only pose table',
                       words=[Path(w).name for w in a.words])
            if a.isolated or selection is not None:
                group = GroupTables(M, temporary)

            def save_tree(tree, suffix):
                path = output.with_name(output.stem + '.' + suffix + '.npy')
                tree.save(path)
                report = tree.summary()
                report.update(predecessor_file=path.name,
                              predecessor_columns=list(tree.dtype.names), key_base=tree.n)
                if a.replay:
                    report['replay'] = model.replay(tree, group, a.replay, a.seed)
                return report

            if a.isolated:
                doc['isolated'] = [save_tree(model.isolated(b, group), f'isolated-{b}')
                                   for b in range(len(model.blocks))]
            if selection is not None:
                tree = model.coupled(selection, group, a.max_states)
                doc['coupled'] = save_tree(tree, 'coupled-' + '-'.join(str(b) for b in selection))
            doc['seconds'] = round(time.time() - t0, 1)
            output.write_text(json.dumps(doc, indent=1) + '\n')
            print('wrote', a.out, flush=True)
        finally:
            if group is not None:
                group.close()
            O.seams.CACHE = old_cache


if __name__ == '__main__':
    main()
