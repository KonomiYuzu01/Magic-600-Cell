"""Exact named-cap covers (DT-J1 M4), beside the research centroid test.

These are reference/cost leads, not a candidate engine.  A cover answer is
about the named pair only; the query has no ordering or next-cap verdict.
Import census first: it isolates the research centroid cache from the repo.
"""

import sys

sys.dont_write_bytecode = True

from collections import OrderedDict
from dataclasses import asdict, dataclass, field
from functools import lru_cache
import operator
from pathlib import Path
from time import perf_counter

from differential.leads import census

import numpy as np
import restore
import seams
from exact import Q5, dot
from sim.kernel import Evaluator, pullback_form
from differential.menu_order import order_digest

THRESHOLD = 10
# n_c . n_d at 49.117... degrees; the next angle is 56.012... degrees.
# Compute the boundary shell from the exact model: 80 neighbours plus self.
MEETING_DOT = sorted((dot(restore.ctx.data.N[0], n) for n in restore.ctx.data.N), reverse=True)[80]


def _cap(c):
    if isinstance(c, (bool, np.bool_)):
        raise ValueError('cap must be an integer in 0..599')
    try:
        c = operator.index(c)
    except TypeError as error:
        raise ValueError('cap must be an integer in 0..599') from error
    if not 0 <= c < 600:
        raise ValueError('cap must be an integer in 0..599')
    return c


@lru_cache(maxsize=600)
def meeting_caps(c):
    """The other 80 caps meeting c, in pole-id order, decided in Q(sqrt(5))."""
    c = _cap(c)
    N = restore.ctx.data.N
    return tuple(d for d in range(600) if d != c and MEETING_DOT <= dot(N[c], N[d]))


def check_meeting():
    """Prove that the research threshold selects the same domain, at every pole."""
    data = restore.ctx.data
    assert MEETING_DOT == Q5(7, 3)
    angle = np.degrees(np.arccos(np.clip(seams.NF @ seams.NF.T / float(data.NN), -1, 1)))
    assert not np.any((angle >= 50) & (angle <= 55.9)), 'pole pair in the threshold gap'
    for c in range(600):
        exact = meeting_caps(c)
        assert len(exact) == 80, f'cap {c}: {len(exact)} meeting caps'
        assert exact == tuple(sorted(int(d) for d in restore.NEAR[c] if d != c)), f'cap {c}'


@dataclass(frozen=True)
class InsideSet:
    input_digest: str
    cap: int
    pieces: np.ndarray = field(repr=False, compare=False)
    blocked: bool


@dataclass
class _Snapshot:
    inside: dict = field(default_factory=dict)
    faces: dict = field(default_factory=dict)
    straddles: dict = field(default_factory=dict)


# Bound memory when a whole census is replayed. Keys are canonical J1 state
# digests, never move counts (undo and replay can revisit a count).
_SNAPSHOTS = OrderedDict()


def _snapshot(S):
    digest = S.st.digest()
    if digest not in _SNAPSHOTS:
        _SNAPSHOTS[digest] = _Snapshot()
        while len(_SNAPSHOTS) > 2:
            _SNAPSHOTS.popitem(last=False)
    _SNAPSHOTS.move_to_end(digest)
    return digest, _SNAPSHOTS[digest]


def inside_sets(S, caps):
    """Full exact certified inside sets, excluding straddlers, cached by digest.

    A blocked cap still has a complete inside set: do not use the restorer's
    early-exit ``inside`` for it. Arrays are read-only to protect the cache.
    """
    digest, cache = _snapshot(S)
    result = {}
    for c in sorted({_cap(c) for c in caps}):
        if c not in cache.inside:
            cls = S.st.classify(c, need_inside=True, stop_at_straddle=False)
            if cls.uncertain:
                raise ValueError(f'cap {c}: exact classification is uncertain')
            pieces = cls.inside.copy()
            pieces.flags.writeable = False
            cache.inside[c] = InsideSet(digest, c, pieces, cls.straddle is not None)
        result[c] = cache.inside[c]
    return result


def _face(S, c, cache):
    if c not in cache.faces:
        if S.inside(c) is None:
            raise ValueError(f'cap {c}: blocked; its face is not defined')
        cache.faces[c] = S.face(c)
    return cache.faces[c]


def _straddlers(S, d, P, inside_d, cache):
    """Every piece of this face straddling d, not just J1's first witness.

    Lattice pieces cannot straddle a lattice cut. Evaluate only the remaining
    face pieces not certified inside; reuse each exact (cut, piece) decision.
    """
    if not inside_d.blocked:
        return ()
    candidates = P[~S.st.lattice_flags()[P] & ~np.isin(P, inside_d.pieces)]
    known = cache.straddles.setdefault(d, {})
    evaluators = {}
    for p in candidates.tolist():
        if p not in known:
            pid = int(S.st.pose_id[p])
            if pid not in evaluators:
                evaluators[pid] = Evaluator(pullback_form(S.st._pose_matrix(pid), S.st.ctx.data.N[d]))
            known[p] = evaluators[pid].classify(S.st.ctx.regions.vform(p))[0] == 'straddle'
    return tuple(sorted(p for p in candidates.tolist() if known[p]))


@dataclass(frozen=True)
class Counts:
    inside: int
    outside: int
    aligned_inside: int
    aligned_outside: int


@dataclass(frozen=True)
class CoverAnswer:
    covers: bool
    counts: Counts
    # None for the float test; the exact test names every face straddler of d.
    straddlers: tuple | None = None

    def as_dict(self):
        return asdict(self)


def _answer(face, selected, straddlers=None):
    _, aligned, side = face
    i = selected & (side > 0)
    o = selected & (side < 0)
    counts = Counts(int(i.sum()), int(o.sum()), int((i & aligned).sum()), int((o & aligned).sum()))
    return CoverAnswer(counts.inside >= THRESHOLD and
                       counts.inside == counts.outside == counts.aligned_inside == counts.aligned_outside,
                       counts, straddlers)


def exact_covers(S, c, d, inside_d):
    """Answer 'd covers c', with counts and c-face pieces that straddle d."""
    c, d = _cap(c), _cap(d)
    digest, cache = _snapshot(S)
    if inside_d.input_digest != digest or inside_d.cap != d:
        raise ValueError('inside set does not match the state digest and covering cap')
    face = _face(S, c, cache)
    P = face[0]
    return _answer(face, np.isin(P, inside_d.pieces), _straddlers(S, d, P, inside_d, cache))


def float_covers(S, c, d, pos):
    """The strict float centroid test in seams.cover_graph, for 'd covers c'."""
    _, cache = _snapshot(S)
    face = _face(S, _cap(c), cache)
    return _answer(face, pos[face[0]] @ seams.NF[_cap(d)] > seams.KAPPA)


@dataclass(frozen=True)
class NotApplicable:
    reason: str
    status: str = 'not_applicable'


@dataclass(frozen=True)
class MeetingAnswer:
    cap: int
    d_covers_c: CoverAnswer
    c_covers_d: CoverAnswer | NotApplicable


@dataclass(frozen=True)
class CoversResult:
    status: str
    input_digest: str
    parameters: dict
    domain: tuple
    answers: tuple
    seconds: float
    message: str | None = None

    def as_dict(self):
        return asdict(self)


def _reason(S, c, classified):
    if classified.blocked:
        return 'blocked'
    n_in, n_out, n_aligned = S.balance(c)
    now = n_in + n_out - 2 * n_aligned
    if now == 0:
        return 'closed'
    if now < 0:
        return 'not misaligned'
    return None


def timed_covers_query(S, c):
    """The named-cap job and its measured parts, for the X4 cost lead.

    Parts stay outside CoversResult: that result exposes only its named
    domain and answers, with no census selection data or additional verdict.
    """
    started = perf_counter()
    c = _cap(c)
    digest, cache = _snapshot(S)
    params = {'cap': c, 'threshold': THRESHOLD, 'menu_identity': S.M.identity,
              'menu_order_digest': order_digest(S.M.record())}
    domain = meeting_caps(c)
    tick = perf_counter()
    classified = inside_sets(S, (c,))
    reason = _reason(S, c, classified[c])
    if reason:
        result = CoversResult('error', digest, params, domain, (), perf_counter() - started,
                              f'cap {c}: {reason} (not misaligned)' if reason == 'closed' else f'cap {c}: {reason}')
        return result, {'inside_sets': perf_counter() - tick, 'faces': 0.0, 'answers': 0.0}
    classified.update(inside_sets(S, domain))
    inside_seconds = perf_counter() - tick
    tick = perf_counter()
    _face(S, c, cache)
    reasons = {d: _reason(S, d, classified[d]) for d in domain}
    for d in domain:
        if reasons[d] is None:
            _face(S, d, cache)
    faces_seconds = perf_counter() - tick
    tick = perf_counter()
    answers = tuple(MeetingAnswer(d, exact_covers(S, c, d, classified[d]),
                                 NotApplicable(reasons[d]) if reasons[d] else
                                 exact_covers(S, d, c, classified[c])) for d in domain)
    answer_seconds = perf_counter() - tick
    return (CoversResult('completed', digest, params, domain, answers, perf_counter() - started),
            {'inside_sets': inside_seconds, 'faces': faces_seconds, 'answers': answer_seconds})


def covers_query(S, c):
    """Every meeting cap's answers about one admissible, misaligned named cap."""
    return timed_covers_query(S, c)[0]


def output_directory(value):
    """Resolve before writing, including existing symlinked parents."""
    path = Path(value).resolve()
    if path.is_relative_to(census.ROOT):
        raise ValueError('--out must be outside the repository')
    return path
