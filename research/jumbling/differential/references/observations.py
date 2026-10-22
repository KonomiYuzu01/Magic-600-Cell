"""Revision-bound J1 observation reference (EH-05, EH-26 and EH-06).

No method scores menu items. Heights extract only the two distances, avoiding
the unused branch calculation in pose_tree. Domain keys hash the sorted exact
images of the 120 unit icosians, the vertices of the 600-cell. Its rotation
group is K+, so this identifies a left coset independently of Domains' ids.

Off-grip means a domain/chamber-pair cell occupied on exactly one side, whose
rotated plane is not H_c for any grip (including blocked grips). A holding cap
contains the *entire closed 3-cell*, decided at every exact vertex. Group
holders are the intersection of the individual cells' holders, not their
union. Chamber ids are in the explicitly supplied domain frame.
"""

import sys

sys.dont_write_bytecode = True

from collections import Counter
from dataclasses import dataclass, fields
from functools import lru_cache
import hashlib
import json

# This import must precede research imports: it isolates the centroid cache.
from differential.leads import census
from differential.leads.x5_release import COUNT_KEYS
from exact import dot, identity, matvec
import numpy as np
import rim
import seams
from sim.kernel import rows_form, sign5
from sim.kplus import to_tuple
import trees


def exact_matrix(matrix):
    return tuple(tuple((x.a, x.b, x.d) for x in row) for row in matrix)


def pose_key(matrix):
    """An exact pose-class identity, independent of pose-table numbering."""
    return json.dumps(exact_matrix(matrix), separators=(',', ':'))


@lru_cache(maxsize=4096)
def domain_key(matrix):
    """SHA-256 of a coset's vertex set, with canonical reduced Q5 triples."""
    images = sorted(tuple((x.a, x.b, x.d) for x in matvec(matrix, v))
                    for v in trees.UNITS)
    return hashlib.sha256(json.dumps(images, separators=(',', ':')).encode('ascii')).hexdigest()


LATTICE_KEY = domain_key(to_tuple(identity(4)))


@dataclass(frozen=True)
class RimSummary:
    status: str
    N_c: int | None = None
    In_c: int | None = None
    Out_c: int | None = None
    A_c: int | None = None
    F_c: int | None = None
    at_floor: bool | None = None

    @property
    def blocked(self):
        return self.status == 'blocked'


@dataclass(frozen=True)
class Observation:
    digest: str
    domain_keys: tuple[str, ...]
    heights: dict[str, tuple[int, int]]
    height_counts: dict[str, int | None]
    rim: dict[int, RimSummary]
    n_total: int
    grip_n: int
    pose_counts: dict[str, int]

    def domain_partition(self):
        groups = {}
        for piece, key in enumerate(self.domain_keys):
            groups.setdefault(key, []).append(piece)
        return {key: tuple(groups[key]) for key in sorted(groups)}


@dataclass(frozen=True)
class Difference:
    field: str
    reference: object
    candidate: object
    piece: int | None = None
    cap: int | None = None
    pose: str | None = None

    def __str__(self):
        where = ''.join(f' {name} {value}' for name, value in
                        (('piece', self.piece), ('cap', self.cap), ('pose', self.pose))
                        if value is not None)
        return f'{self.field}{where}: reference={self.reference!r}, candidate={self.candidate!r}'


def left_counts(histogram):
    """X5's bins and maximum; retain sparse odd bins for the S4 references."""
    out = dict.fromkeys(COUNT_KEYS, 0)
    out['max_above_6'] = max((h for h, n in histogram.items() if h > 6 and n), default=None)
    for height, count in sorted(histogram.items()):
        if height < 0:
            raise ValueError(f'negative tree height {height}')
        key = 'n_above_6' if height > 6 else f'n_{height}'
        out[key] = out.get(key, 0) + count
    return out


def _cap(c):
    if type(c) is not int or not 0 <= c < 600:
        raise ValueError('cap must be an integer in 0..599')
    return c


def _rim_value(domains, c, classification, route=None):
    if classification.status != 'admissible':
        return RimSummary(classification.status)
    mask = np.zeros(rim.NP, bool)
    mask[classification.inside] = True
    if route is None:
        rs = domains.rim_sets(c, mask)
        n_in = sum(len(a) for _, a, _ in rs.values())
        n_out = sum(len(b) for _, _, b in rs.values())
        aligned = sum(len(a & b) for _, a, b in rs.values())
        n = domains.misalignment(c, mask, rs=rs)
    else:
        n_in, n_out, aligned = route.balance(c)
        n = n_in + n_out - 2 * aligned
    floor = abs(n_in - n_out)
    return RimSummary('admissible', n, n_in, n_out, aligned, floor, n == floor)


def observe(state, *, caps=None, rim_route='rim'):
    """Observe one state, without mutation or an item loop.

    caps selects the returned rim rows. N and grip_n always cover the entire
    state, so even a named-cap query cannot mislabel a partial sum as grip N.
    compare requires a full 600-cap reference; subset observations are queries.
    """
    if rim_route not in ('rim', 'seams'):
        raise ValueError("rim_route must be 'rim' or 'seams'")
    selected = set(range(600) if caps is None else (_cap(c) for c in caps))
    keys = {}
    heights = {}
    counts = Counter()
    pose_counts = Counter()
    for pid, count in zip(*np.unique(state.pose_id, return_counts=True)):
        pid = int(pid)
        pose = state._poses[pid]
        matrix = state._pose_matrix(pid)
        keys[pid] = LATTICE_KEY if pose.kidx >= 0 else domain_key(to_tuple(matrix))
        dl, dr = ((0, 0) if pose.kidx >= 0 else
                  tuple(trees.dist(factor) for factor in trees.factors(matrix)))
        heights[pose_key(matrix)] = (dl, dr)
        pose_counts[pose_key(matrix)] += int(count)
        counts[dl] += int(count)
    domains = rim.Domains(state)
    route = seams.SeamRestorer(state, state.menu or census.menu()) if rim_route == 'seams' else None
    if route is not None:
        route._dom = domains
    rows, grip_n = {}, 0
    for c in range(600):
        cl = state.classify(c, need_inside=True, stop_at_straddle=True)
        if route is not None:
            route._cls[c] = cl.inside if cl.status == 'admissible' else None
        row = _rim_value(domains, c, cl, route)
        if c in selected:
            rows[c] = row
        if row.N_c is not None:
            grip_n += row.N_c
    return Observation(state.digest(), tuple(keys[int(pid)] for pid in state.pose_id),
                       heights, left_counts(counts), rows, domains.total_misalignment(), grip_n,
                       dict(pose_counts))


def height_histograms(state, observation):
    """Both recorded trees-fixtures histograms (only occupied pose classes)."""
    pieces, classes = Counter(), Counter()
    for pid, count in zip(*np.unique(state.pose_id, return_counts=True)):
        dl, dr = observation.heights[pose_key(state._pose_matrix(int(pid)))]
        key = f'{dl},{dr}'
        pieces[key] += int(count)
        classes[key] += 1
    return {'pieces_by_heights': dict(sorted(pieces.items())),
            'pose_classes_by_heights': dict(sorted(classes.items()))}


@dataclass(frozen=True)
class CutCell:
    domain_key: str
    frame: tuple
    pole: int
    chamber_pair: tuple[int, int]
    state: str


def cut_cells(state, c):
    """Occupied cells of a named admissible cut, once per domain and cell."""
    c = _cap(c)
    cl = state.classify(c, need_inside=True, stop_at_straddle=True)
    if cl.status != 'admissible':
        raise ValueError(f'cap {c} is {cl.status}; cut cells require an admissible cap')
    domains = rim.Domains(state)
    mask = np.zeros(rim.NP, bool)
    mask[cl.inside] = True
    out = []
    for d, (e, ins, outside) in domains.rim_sets(c, mask).items():
        key = domain_key(to_tuple(domains.reps[d]))
        frame = exact_matrix(domains.reps[d])
        cells = sorted(ins | outside)
        partners = rim.partner(cells, np.full(len(cells), e), add=True)
        for lower, upper in zip(cells, partners.tolist()):
            if upper < 0:
                raise AssertionError(f'cap {c}, domain {key}, cell {lower}: no upper chamber')
            status = ('matched' if lower in ins and lower in outside else
                      'inside_only' if lower in ins else 'outside_only')
            out.append(CutCell(key, frame, e, (lower, upper), status))
    return tuple(sorted(out, key=lambda row: (row.domain_key, row.chamber_pair)))


@dataclass(frozen=True)
class OffGripCell:
    domain_key: str
    frame: tuple
    hyperplane: tuple  # (normal of four Q5 triples, offset Q5 triple)
    chamber_pair: tuple[int, int]
    holding_caps: dict[int, str]


def _holders(vertices):
    """All caps containing a closed cell. Object arrays use unbounded ints."""
    denominator, rows = rows_form(vertices)
    a = np.array(rows, dtype=object)
    x, z = a[:, :4], a[:, 4:]
    n1, n2 = rim.data.N1.astype(object), rim.data.N2.astype(object)
    p = 125 * (n1 @ x.T + 5 * (n2 @ z.T)) - 2 * denominator * 1452
    q = 125 * (n1 @ z.T + n2 @ x.T) - 2 * denominator * 484
    return tuple(c for c in range(600)
                 if all(sign5(int(a), int(b)) >= 0 for a, b in zip(p[c], q[c])))


def off_grip_seams(state):
    """Domain key -> cells, with exact planes and per-cell holding-cap status.

    N counts domain/cell pairs, including an occupied lower chamber of a cell
    whose upper chamber is absent. Neither those cells nor blocked grip cuts
    may be discarded. Off-grip is a geometric test, never N minus a survey.
    """
    domains = rim.Domains(state)
    lo, up = rim.cell_pairs()
    grouped, statuses = {}, {}
    for d, members in enumerate(domains.members):
        held = np.zeros(rim.NP + 1, bool)
        held[domains.cham[members]] = True
        if d == 0:
            held[rim.CORE] = True
        frame = domains.reps[d]
        key = domain_key(to_tuple(frame))
        normals = {}
        for index in np.flatnonzero(held[lo] != held[up]).tolist():
            lower, upper = int(lo[index]), int(up[index])
            signature = rim.data.signature(upper)
            removed = signature - (rim.data.signature(lower) if lower != rim.CORE else set())
            if len(removed) != 1:
                raise AssertionError(f'not a cut cell: {(lower, upper)}')
            e = next(iter(removed))
            if e not in normals:
                normals[e] = tuple(matvec(frame, rim.data.N[e]))
            normal = normals[e]
            # All normals have the same norm and the nonzero offset is fixed:
            # equality of normals is exactly equality of these affine planes.
            if normal in rim.data.POLE:
                continue
            vertices = [matvec(frame, v) for v in rim.reg.vertices(upper)
                        if dot(rim.data.N[e], v) == rim.data.KAPPA]
            if len(vertices) < 4:
                raise AssertionError(f'cut cell {(lower, upper)} has fewer than four vertices')
            caps = _holders(vertices)
            for c in caps:
                if c not in statuses:
                    statuses[c] = state.classify(c, need_inside=False, stop_at_straddle=True).status
            plane = (tuple((x.a, x.b, x.d) for x in normal),
                     (rim.data.KAPPA.a, rim.data.KAPPA.b, rim.data.KAPPA.d))
            grouped.setdefault(key, []).append(OffGripCell(
                key, exact_matrix(frame), plane, (lower, upper), {c: statuses[c] for c in caps}))
    return {key: tuple(sorted(cells, key=lambda cell: cell.chamber_pair))
            for key, cells in sorted(grouped.items())}


RIM_VALUES = ('N_c', 'In_c', 'Out_c', 'A_c', 'F_c', 'at_floor')


def _gap(reference):
    """The first gap of a reference, or None when it is complete and self-consistent.

    Every piece has a domain key; the pose classes of `pose_counts` hold every
    piece and are exactly those of `heights`; the counts by left height follow
    from them; all 600 caps have a row whose values its status requires; and
    grip N is the sum of N_c over the admissible caps.
    """
    if len(reference.domain_keys) != rim.NP:
        return 'pieces'
    counts = reference.pose_counts
    if (set(counts) != set(reference.heights) or sum(counts.values()) != rim.NP
            or any(type(n) is not int or n <= 0 for n in counts.values())):
        return 'pose classes'
    left = Counter()
    try:
        for pose, n in counts.items():
            left[reference.heights[pose][0]] += n
        if left_counts(left) != reference.height_counts:
            return 'height counts'
    except (TypeError, IndexError, ValueError):
        return 'heights'
    if set(reference.rim) != set(range(600)):
        return 'caps'
    for c in range(600):
        row = reference.rim[c]
        values = [getattr(row, name) for name in RIM_VALUES]
        if row.status != 'admissible':
            if any(value is not None for value in values):
                return f'cap {c}'
        elif (any(value is None for value in values) or row.F_c != abs(row.In_c - row.Out_c)
              or row.at_floor != (row.N_c == row.F_c)):
            return f'cap {c}'
    if reference.n_total is None or reference.grip_n != sum(
            row.N_c for row in reference.rim.values() if row.status == 'admissible'):
        return 'N'
    return None


def compare(candidate: Observation, reference: Observation) -> Difference | None:
    """First difference: digest, piece partition, heights, pose counts, counts, cap, N.

    A full reference is required (`_gap`). Missing candidate entries are
    differences; comparing two truncated inputs must never be reported as a pass.
    """
    gap = _gap(reference)
    if gap is not None:
        raise ValueError(f'comparison requires a complete reference: {gap}')
    if candidate.digest != reference.digest:
        return Difference('digest', reference.digest, candidate.digest)
    for piece in range(max(len(candidate.domain_keys), len(reference.domain_keys))):
        actual = candidate.domain_keys[piece] if piece < len(candidate.domain_keys) else None
        expected = reference.domain_keys[piece] if piece < len(reference.domain_keys) else None
        if actual != expected:
            return Difference('partition', expected, actual, piece=piece)
    for pose in sorted(set(candidate.heights) | set(reference.heights)):
        actual, expected = candidate.heights.get(pose), reference.heights.get(pose)
        if actual != expected:
            return Difference('heights', expected, actual, pose=pose)
    for pose in sorted(set(candidate.pose_counts) | set(reference.pose_counts)):
        actual, expected = candidate.pose_counts.get(pose), reference.pose_counts.get(pose)
        if actual != expected:
            return Difference('pose_counts', expected, actual, pose=pose)
    order = (*COUNT_KEYS, 'max_above_6', *sorted(
        (set(candidate.height_counts) | set(reference.height_counts)) - set(COUNT_KEYS) - {'max_above_6'}))
    for key in order:
        actual, expected = candidate.height_counts.get(key), reference.height_counts.get(key)
        if (key not in candidate.height_counts or key not in reference.height_counts or actual != expected):
            return Difference(f'height_counts.{key}', expected, actual)
    for c in sorted(set(candidate.rim) | set(reference.rim)):
        actual, expected = candidate.rim.get(c), reference.rim.get(c)
        if actual is None or expected is None:
            return Difference('rim', expected, actual, cap=c)
        for field in fields(RimSummary):
            a, b = getattr(actual, field.name), getattr(expected, field.name)
            if a != b:
                return Difference(f'rim.{field.name}', b, a, cap=c)
    for key in ('n_total', 'grip_n'):
        actual, expected = getattr(candidate, key), getattr(reference, key)
        if actual != expected:
            return Difference(key, expected, actual)
    return None
