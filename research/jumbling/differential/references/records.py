"""Revision-bound readers and exact J1 input refusals (EN2-K).

No candidate module is imported. Store records are plain data; their identities
are read using the store's recorded_at/menu/menu_order spelling. Missing
identities are mismatches, never inferred from the current revision.
"""

import sys

sys.dont_write_bytecode = True

from collections.abc import Mapping
from dataclasses import dataclass
import gzip
import hashlib
import json
from pathlib import Path


class RecordMismatch(ValueError):
    def __init__(self, field, expected, actual):
        self.field, self.expected, self.actual = field, expected, actual
        super().__init__(f'{field}: expected {expected!r}, recorded {actual!r}')


class IncompleteRecordError(RecordMismatch):
    """An interrupted or failed job has no completed result to read."""


@dataclass(frozen=True)
class LibraryRecord:
    data: dict
    library_digest: str


def _get(value, key, default=None):
    return value.get(key, default) if isinstance(value, Mapping) else getattr(value, key, default)


def read_against(record, *, input_digest=None, menu_identity=None, order_digest=None,
                 contract_revision=None, library_digest=None):
    """Return the validated record, raising at the first mismatching expectation.

    A library path reads the gzip's *decompressed bytes* and hashes those exact
    bytes, not a reserialisation of the JSON. A LibraryRecord retains that
    evidence alongside the parsed library. Job results are returned as typed
    results, never converted to lists (including an empty list on interruption).
    Expectations not supplied by the caller impose no additional identity.
    """
    if isinstance(record, (str, Path)):
        path = Path(record)
        raw = gzip.decompress(path.read_bytes()) if path.suffix == '.gz' else path.read_bytes()
        data = json.loads(raw)
        if not isinstance(data, dict):
            raise RecordMismatch('format', 'correction-library object', type(data).__name__)
        record = LibraryRecord(data, hashlib.sha256(raw).hexdigest())
    status = _get(record, 'status')
    if status is not None and status != 'completed':
        raise IncompleteRecordError('status', 'completed', status)
    parameters = _get(record, 'parameters', {})
    identities = _get(record, 'identities', {})
    actual = {
        'input_digest': _get(record, 'input_digest', _get(record, 'recorded_at')),
        'menu_identity': _get(record, 'menu_identity',
                              _get(parameters, 'menu_identity', _get(identities, 'menu'))),
        'order_digest': _get(record, 'order_digest', _get(parameters, 'order_digest',
                             _get(parameters, 'menu_order_digest', _get(identities, 'menu_order')))),
        'contract_revision': _get(record, 'contract_revision',
                                  _get(parameters, 'contract_revision', _get(identities, 'contract_revision'))),
        'library_digest': _get(record, 'library_digest', _get(identities, 'library_digest')),
    }
    expectations = dict(input_digest=input_digest, menu_identity=menu_identity, order_digest=order_digest,
                        contract_revision=contract_revision, library_digest=library_digest)
    for field, expected in expectations.items():
        if expected is not None and (type(actual[field]) is not type(expected) or actual[field] != expected):
            raise RecordMismatch(field, expected, actual[field])
    return record


class MenuClosureError(ValueError):
    def __init__(self, condition, index, *, conjugator=None):
        self.condition, self.index, self.conjugator = condition, index, conjugator
        detail = '' if conjugator is None else f', A4 item {conjugator}'
        super().__init__(f'menu item {index}: {condition}{detail}')


def check_menu_closure(record):
    """Check exact inverses, A4 conjugation and A4 inclusion, in that order.

    The index for a missing A4 element is its index in J1's A4_0 list; for
    other errors it is the first offending index in the supplied base record.
    Boolean claims in a record cannot stand in for these matrix checks.
    J1 subsequently validates the record's identity and its claimed flags too.
    """
    from differential.leads import census  # isolate the cache before research imports
    from exact import matmul, matvec, transpose
    from sim.kplus import is_rotation, matrix_from_json, to_tuple
    from sim.twists import TwistError, TwistMenu

    ctx = census.restore.ctx
    if not isinstance(record, Mapping) or not isinstance(record.get('base'), list) or not record['base']:
        raise MenuClosureError('base must be a non-empty list', 0)
    matrices = []
    for index, item in enumerate(record['base']):
        try:
            matrix = to_tuple(matrix_from_json(item['matrix']))
            if not is_rotation(matrix) or matvec(matrix, ctx.data.N[0]) != list(ctx.data.N[0]):
                raise ValueError('not a rotation fixing cap 0')
            matrices.append(matrix)
        except (KeyError, TypeError, ValueError, IndexError) as error:
            raise MenuClosureError('invalid exact rotation', index) from error
    keys = set(matrices)
    for index, matrix in enumerate(matrices):
        if to_tuple(transpose(matrix)) not in keys:
            raise MenuClosureError('not inverse-closed', index)
    stab = [ctx.kplus.matrix(k) for k in ctx.kplus.stab0]
    for index, matrix in enumerate(matrices):
        for j, s in enumerate(stab):
            if to_tuple(matmul(matmul(s, matrix), transpose(s))) not in keys:
                raise MenuClosureError('not A4-conjugation-invariant', index, conjugator=j)
    for index, matrix in enumerate(stab):
        if matrix not in keys:
            raise MenuClosureError('does not contain A4', index)
    try:
        return TwistMenu.from_record(ctx, record)
    except (TwistError, KeyError, TypeError, ValueError) as error:
        raise MenuClosureError(f'invalid record: {error}', 0) from error


class TwistRefused(ValueError):
    def __init__(self, cap, status, reason):
        self.cap, self.status, self.reason = cap, status, reason
        super().__init__(f'cap {cap}: {status}: {reason}')


def request_twist(state, cap, axis, degrees, *, menu):
    """Map A1 input through J1 and return a new state only on exact admission.

    Work is performed on a copy, so even a failure during construction or
    classification cannot alter the caller's pose table, journal or caches.
    """
    from differential.leads import census  # noqa: F401
    from sim.twists import TwistError, UnrepresentableTwist, cayley_axis_angle
    from .word_check import copy_state

    trial = copy_state(state)
    try:
        if type(cap) is not int or not 0 <= cap < 600:
            raise TwistError('cap must be an integer in 0..599')
        twist, report = cayley_axis_angle(trial.ctx, cap, axis, degrees, menu=menu)
    except (TwistError, TypeError, ValueError, OverflowError) as error:
        raise TwistRefused(cap, 'unrepresentable', str(error)) from error
    if isinstance(twist, UnrepresentableTwist):
        raise TwistRefused(cap, 'uncertain', twist.reason)
    if not menu.contains(twist):
        raise TwistRefused(cap, 'invalid', 'rotation is not an exact menu element')
    outcome = trial.apply(twist)
    if not outcome.applied:
        raise TwistRefused(cap, outcome.status, outcome.reason or 'twist is not admissible')
    return trial, report
