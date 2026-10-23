"""Exact, bounded, in-memory acceleration of :mod:`rim`'s domain machinery.

Shared caches contain matrices and value-to-value operations only. Pose ids and
domain indexes are local to a Domains instance. Even the local coset cache uses
the exact twist matrix: no assumption about a caller's gkey or menu is needed.
Rim counting follows rim, with an additional cache of exact chamber images.
"""
from collections import OrderedDict
import sys

import numpy as np

import rim
from exact import Q5, identity, matmul, matvec, transpose
from sim.kplus import to_tuple


class _Matrix:
    """An exact value with a cached hash, never a pose or domain number.

    State treats its Q5 matrices as immutable; this private wrapper does too.
    Hash equality is only a dictionary pre-filter: equality checks every entry.
    """

    __slots__ = ('rows', '_hash')

    def __init__(self, rows):
        self.rows = rows
        self._hash = hash(rows)

    def __hash__(self):
        return self._hash

    def __eq__(self, other):
        return isinstance(other, _Matrix) and self.rows == other.rows


def _matrix(m):
    if isinstance(m, _Matrix):
        return m
    rows = m if isinstance(m, tuple) and all(isinstance(r, tuple) for r in m) else to_tuple(m)
    key = _Matrix(rows)
    cache = _CACHES['matrix']
    known = cache.get(key)
    return cache.put(key, key) if known is None else known


def _bytes(value, seen):
    """Retained Python/NumPy bytes, counting shared objects once per traversal."""
    ident = id(value)
    if ident in seen:
        return 0
    seen.add(ident)
    size = sys.getsizeof(value)
    if isinstance(value, dict):
        size += sum(_bytes(k, seen) + _bytes(v, seen) for k, v in value.items())
    elif isinstance(value, (tuple, list, set, frozenset)):
        size += sum(_bytes(v, seen) for v in value)
    elif isinstance(value, Q5):
        size += sum(_bytes(v, seen) for v in (value.a, value.b, value.d))
    elif isinstance(value, _Matrix):
        size += _bytes(value.rows, seen) + _bytes(value._hash, seen)
    return size


class _Cache:
    """Count-bounded LRU; byte accounting stays outside the hot path."""

    def __init__(self, max_entries):
        self.max_entries = max_entries
        self.values = OrderedDict()
        self.hits = self.misses = self.evictions = 0

    def get(self, key):
        try:
            value = self.values[key]
        except KeyError:
            self.misses += 1
            return None
        self.values.move_to_end(key)
        self.hits += 1
        return value

    def put(self, key, value):
        if self.max_entries < 1:
            return value
        self.values[key] = value
        self.values.move_to_end(key)
        while len(self.values) > self.max_entries:
            self.values.popitem(last=False)
            self.evictions += 1
        return value

    def clear(self):
        self.values.clear()
        self.hits = self.misses = self.evictions = 0


_CACHES = {
    'matrix': _Cache(65536),
    'product': _Cache(32768),
    'relative': _Cache(65536),
    'pole': _Cache(32768),
    'float': _Cache(8192),
    'match': _Cache(32768),
    # A K+ composition row is 7200 int16 values (14,400 bytes), independent
    # of any state. Even caching every row costs only about 104 MB.
    'composition': _Cache(7200),
    'cut_cells': _Cache(600),
}
_I = _matrix(identity(4))
_NF = np.array([[float(x) for x in row] for row in rim.data.N])
_BF = _NF[rim.kp.basis].T
# A vectorized hash of rounded float pole coordinates proposes candidates only.
# Hash/rounding collisions and missed buckets lead to the exact ordered fallback.
_WEIGHTS = np.array([0x9E3779B185EBCA87, 0xC2B2AE3D27D4EB4F,
                     0x165667B19E3779F9, 0x85EBCA77C2B2AE63], np.uint64)


def _float_hash(rows):
    return (np.rint(rows * 1e7).astype(np.int64).view(np.uint64) * _WEIGHTS).sum(axis=-1, dtype=np.uint64)


_FH = _float_hash(_NF)
_FO = np.argsort(_FH)
_FS = _FH[_FO]
_CHAMBERS = np.arange(rim.NP, dtype=np.int64)


def clear_caches():
    """Discard shared work; existing Domains instances remain valid."""
    for cache in _CACHES.values():
        cache.clear()


def cache_info():
    """Cache counts and retained bytes, deduplicated across all shared caches."""
    seen, out = set(), {}
    for name, cache in _CACHES.items():
        out[name] = {'size': len(cache.values), 'max_entries': cache.max_entries,
                     'bytes': _bytes(cache.values, seen),
                     'hits': cache.hits, 'misses': cache.misses, 'evictions': cache.evictions}
    return out


def _product(a, b):
    a, b = _matrix(a), _matrix(b)
    cache, key = _CACHES['product'], (a, b)
    result = cache.get(key)
    if result is None:
        result = cache.put(key, _matrix(matmul(a.rows, b.rows)))
    return result


def _relative_index(r, m):
    r, m = _matrix(r), _matrix(m)
    cache, key = _CACHES['relative'], (r, m)
    result = cache.get(key)
    if result is None:
        result = cache.put(key, rim.kp.index_of_matrix(to_tuple(matmul(transpose(r.rows), m.rows))))
    return result


def _floating(m):
    m = _matrix(m)
    cache = _CACHES['float']
    result = cache.get(m)
    if result is None:
        try:
            result = np.array([[float(x) for x in row] for row in m.rows])
        except (OverflowError, ValueError):
            result = np.full((4, 4), np.nan)
        result.setflags(write=False)
        cache.put(m, result)
    return result


def _chamber_cells(k, cells):
    cache = _CACHES['composition']
    row = cache.get(k)
    if row is None:
        order = len(rim.kp.perms)
        row = rim.kp.compose_vec(np.full(order, k, np.int64), np.arange(order)).astype(np.int16)
        row.setflags(write=False)
        cache.put(k, row)
    q = np.fromiter(cells, np.int64, count=len(cells))
    core = q == rim.CORE
    q = q[~core]
    images = rim.piece_of[rim.orbit_of[q], row[rim.transport[q]]].tolist()
    if core.any():
        images.append(rim.CORE)
    return images


def _cut_cells(e):
    """Exact chamber lists for the two separate predicates in rim.rim_sets.

    Build the outside list independently, using precisely rim's partner test;
    no extra assumption about signature hashes or an inverse relation is needed.
    """
    cache = _CACHES['cut_cells']
    result = cache.get(e)
    if result is None:
        up = rim.data.cap_members(e)
        low = rim.partner(up, np.full(len(up), e), add=False)
        ok = low >= 0
        upper, lower_in = up[ok].copy(), low[ok]
        outside = _CHAMBERS[~rim.data.in_sig(_CHAMBERS, np.full(rim.NP, e))]
        lower_out = outside[rim.partner(outside, np.full(len(outside), e), add=True) >= 0]
        result = (upper, lower_in, lower_out)
        for array in result:
            array.setflags(write=False)
        cache.put(e, result)
    return result


class Domains(rim.Domains):
    """rim.Domains with identical representatives, chambers and exact decisions."""

    def __init__(self, st):
        pids = np.unique(st.pose_id).tolist()
        self.reps = [identity(4)]
        self._repkeys = [_I]
        self._repindex = {_I: 0}
        self._rf = [_floating(_I).T]
        dom_of = np.zeros(len(st._poses), np.int64)
        k_of = np.zeros(len(st._poses), np.int64)
        # Keep precisely rim's increasing-pose order and identity-first rule.
        for pid in pids:
            pose = st._poses[pid]
            if pose.kidx >= 0:
                k_of[pid] = pose.kidx
                continue
            m = _matrix(st._pose_matrix(pid))
            d, k = self._match(m)
            if d < 0:
                d, k = len(self.reps), rim.IDK
                self.reps.append(st._pose_matrix(pid))
                self._repkeys.append(m)
                self._repindex[m] = d
                self._rf.append(_floating(m).T)
            dom_of[pid], k_of[pid] = d, k
        # One vectorized chamber map replaces a full piece scan for each pose.
        self.dom = dom_of[st.pose_id]
        ks = rim.kp.compose_vec(k_of[st.pose_id], rim.transport)
        self.cham = rim.piece_of[rim.orbit_of, ks]
        order = np.argsort(self.dom, kind='stable')
        ends = np.cumsum(np.bincount(self.dom, minlength=len(self.reps)))
        self.members = list(np.split(order, ends[:-1]))
        # Local answers may contain domain indexes; these never leave this object.
        self._pole = _Cache(4096)
        self._coset = _Cache(4096)
        self._twists = _Cache(4096)
        # At most 32 dense int32 chamber maps: about 23 MB per live object.
        self._inverse = _Cache(32)

    def _proposals(self, m):
        with np.errstate(over='ignore', invalid='ignore'):
            images = np.matmul(np.asarray(self._rf), _floating(m) @ _BF)
        if not np.isfinite(images).all() or np.any(np.abs(images) > 1e10):
            return []
        hashes = _float_hash(images.transpose(0, 2, 1))
        pos = np.minimum(np.searchsorted(_FS, hashes), len(_FS) - 1)
        rows = np.flatnonzero((_FS[pos] == hashes).all(axis=1))
        if not len(rows):
            return []
        ks = rim.kp.index_of_images(_FO[pos[rows]])
        return rows[ks >= 0].tolist()

    def _match(self, m):
        # A shared hit names an exact representative, never its former index.
        # If that representative is absent here, continue with the exact search.
        known = _CACHES['match'].get(m)
        if known is not None:
            r, k = known
            j = self._repindex.get(r)
            if j is not None:
                return j, k
        tested = set()
        for j in self._proposals(m):
            tested.add(j)
            k = _relative_index(self._repkeys[j], m)
            if k >= 0:
                # Distinct left cosets have no overlap: an exact success is
                # necessarily the same representative rim's ordered scan finds.
                _CACHES['match'].put(m, (self._repkeys[j], k))
                return j, k
        for j, r in enumerate(self._repkeys):
            if j in tested:
                continue
            k = _relative_index(r, m)
            if k >= 0:
                _CACHES['match'].put(m, (r, k))
                return j, k
        return -1, None

    def pole(self, d, c):
        key = (d, c)
        result = self._pole.get(key)
        if result is None:
            shared_key = (self._repkeys[d], c)
            shared = _CACHES['pole']
            result = shared.get(shared_key)
            if result is None:
                w = matvec(transpose(self._repkeys[d].rows), rim.data.N[c])
                result = shared.put(shared_key, rim.data.POLE.get(tuple(w), -1))
            self._pole.put(key, result)
        return result

    def coset(self, gkey, g, d):
        rows = g if isinstance(g, tuple) and all(isinstance(r, tuple) for r in g) else to_tuple(g)
        matrix = self._twists.get(gkey)
        if matrix is None or matrix.rows != rows:
            matrix = self._twists.put(gkey, _matrix(rows))
        key = (matrix, d)
        result = self._coset.get(key)
        if result is None:
            result = self._match(_product(matrix, self._repkeys[d]))
            self._coset.put(key, result)
        return result

    def rim_sets(self, c, inside):
        out = {}
        for d, mem in enumerate(self.members):
            e = self.pole(d, c) if mem.size else -1
            if e < 0:
                continue
            inverse = self._inverse.get(d)
            if inverse is None:
                inverse = np.full(rim.NP, -1, np.int32)
                q = self.cham[mem]
                inverse[q] = mem
                # Keep the reference semantics even for a non-injective input.
                if not np.array_equal(inverse[q], mem):
                    inverse = False
                else:
                    inverse.setflags(write=False)
                self._inverse.put(d, inverse)
            if inverse is False:
                return super().rim_sets(c, inside)
            upper, lower_in, lower_out = _cut_cells(e)
            pieces = inverse[upper]
            held = pieces >= 0
            cells_in = set(lower_in[held][inside[pieces[held]]].tolist())
            pieces = inverse[lower_out]
            held = pieces >= 0
            cells_out = set(lower_out[held][~inside[pieces[held]]].tolist())
            if d == 0:
                cells_out.add(rim.CORE)
            out[d] = (e, cells_in, cells_out)
        return out

    def misalignment(self, c, inside, gkey='I', g=None, rs=None):
        """rim's count, reusing only exact, state-independent chamber images."""
        rs = rs if rs is not None else self.rim_sets(c, inside)
        if g is None:
            return sum(len(a ^ b) for _, a, b in rs.values())
        got = {d: set() for d in rs}
        bad = 0
        for d, (_, cells_in, _) in rs.items():
            if not cells_in:
                continue
            d2, k = self.coset(gkey, g, d)
            if d2 < 0 or d2 not in rs:
                bad += len(cells_in)
                continue
            got[d2].update(_chamber_cells(k, cells_in))
        return bad + sum(len(got[d] ^ rs[d][2]) for d in rs)
