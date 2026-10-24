"""Full certified straddler sets of a named cap, without changing J1.

Explicit private J1 dependencies: State._kpose, _groups, _anchors, _pose_matrix,
_eval and _straddle_certificate. The traversal follows J1's pose groups and
anchor supersets but retains every straddler. The separate checker parses the
points anew and tests the current posed region and cut exactly; it trusts
neither cached h values nor the certificate's vertex indices or scan order.
"""

import sys

sys.dont_write_bytecode = True

from collections.abc import Mapping
from time import perf_counter

import numpy as np

from exact import Q5
from sim.kernel import Evaluator, pullback_form
from sim.state import Classification, posed_point_check


def straddler_set(state, cap, *, budget=None) -> dict:
    """Return every certified straddler and classification counts and seconds.

    A finite exact-evaluation budget may leave pieces uncertain. Such pieces
    contribute only to ``uncertain_pieces``, never to a clear or straddler count;
    the result's status is then ``uncertain`` even if some straddlers are known.
    ``blocked`` always describes the certified pieces, not unknown ones.
    """
    if type(cap) is not int or not 0 <= cap < 600:
        raise ValueError('cap must be an integer in 0..599')
    if budget is not None and (type(budget) is not int or budget < 0):
        raise ValueError('budget must be a non-negative integer or None')
    started = perf_counter()
    data, reg = state.ctx.data, state.ctx.regions
    cl = Classification(cap)
    lattice = state._kpose()[state.pose_id] >= 0
    cl.counts['signature'] = int(lattice.sum())
    off = np.flatnonzero(~lattice)
    certificates = {}
    for pid, members in (state._groups(off) if off.size else []):
        ev = Evaluator(pullback_form(state._pose_matrix(pid), data.N[cap]))
        for anchor, sub in state._anchors(pid, members):
            if budget is not None and cl.exact_evaluations >= budget:
                cl.uncertain += len(sub)
                continue
            result = state._eval(ev, reg.cap_vform(anchor),
                                 lambda a=anchor: reg.cap_frows(a), cl, budget)
            if result[0] == 'uncertain':
                cl.uncertain += len(sub)
                continue
            if result[0] != 'straddle':
                cl.counts['group_superset'] += len(sub)
                continue
            for piece in sub.tolist():
                result = state._eval(ev, reg.vform(piece),
                                     lambda p=piece: reg.frows(p), cl, budget)
                if result[0] == 'uncertain':
                    cl.uncertain += 1
                elif result[0] == 'straddle':
                    certificates[piece] = state._straddle_certificate(
                        piece, pid, cap, result[2], result[3])
                else:
                    cl.counts['piece_vertices'] += 1
    pieces = sorted(certificates)
    counts = dict(cl.counts, straddlers=len(pieces))
    return {'blocked': bool(pieces), 'pieces': pieces,
            'certificates': {p: certificates[p] for p in pieces},
            'status': 'uncertain' if cl.uncertain else ('blocked' if pieces else 'admissible'),
            'uncertain_pieces': cl.uncertain, 'counts': counts,
            'exact_evaluations': cl.exact_evaluations,
            'filtered_evaluations': cl.filtered_evaluations,
            'seconds': perf_counter() - started}


def _point(value):
    if not isinstance(value, (list, tuple)) or len(value) != 4:
        raise ValueError('a point needs four exact Q(sqrt5) coordinates')
    point = []
    for triple in value:
        if (not isinstance(triple, (list, tuple)) or len(triple) != 3
                or any(type(x) is not int for x in triple) or triple[2] <= 0):
            raise ValueError('a coordinate needs integer [a, b, d] with d > 0')
        point.append(Q5(*triple))
    return point


def check_certificate(state, cap, piece, cert) -> bool:
    """Independently check two exact points against this state's piece and cut."""
    if (type(cap) is not int or not 0 <= cap < 600 or type(piece) is not int
            or not 0 <= piece < len(state.pose_id) or not isinstance(cert, Mapping)):
        return False
    if (type(cert.get('piece')) is not int or cert['piece'] != piece
            or type(cert.get('grip')) is not int or cert['grip'] != cap):
        return False
    try:
        below, above = _point(cert['point_below']), _point(cert['point_above'])
    except (KeyError, TypeError, ValueError):
        return False
    data = state.ctx.data
    pose = state._pose_matrix(int(state.pose_id[piece]))
    signature = data.signature(piece)
    return bool(posed_point_check(data, signature, pose, below)
                and posed_point_check(data, signature, pose, above)
                and data.h(cap, below).sign() < 0 < data.h(cap, above).sign())
