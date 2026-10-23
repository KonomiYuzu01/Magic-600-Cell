"""Exact menu-record encodings, without loading the reference engine."""

from collections.abc import Mapping
import hashlib
import json


I_A_IDENTITY = 'fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c'
I_A_ORDER_DIGEST = '5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c'


class MenuRecordError(ValueError):
    """A menu record cannot be read as exact base matrices."""


class MenuIdentityError(MenuRecordError):
    """The exact matrices disagree with a stated menu identity."""


class MenuOrderError(MenuRecordError):
    """The base-item order disagrees with its expected digest."""


def item_key(matrix) -> str:
    """Compact JSON of the stored matrix, preserving its exact integer triples."""
    if not isinstance(matrix, list) or len(matrix) != 4:
        raise MenuRecordError('matrix must be a list of four rows')
    for row in matrix:
        if not isinstance(row, list) or len(row) != 4:
            raise MenuRecordError('matrix rows must be lists of four entries')
        for entry in row:
            if (not isinstance(entry, list) or len(entry) != 3
                    or any(type(n) is not int for n in entry) or entry[2] <= 0):
                raise MenuRecordError('matrix entries must be [a, b, d] integer lists with d > 0')
    return json.dumps(matrix, separators=(',', ':'))


def _keys(record):
    if not isinstance(record, Mapping):
        raise MenuRecordError('menu record must be a mapping')
    base = record.get('base')
    if not isinstance(base, list) or not base:
        raise MenuRecordError('menu base must be a non-empty list')
    keys = []
    for index, item in enumerate(base):
        if not isinstance(item, Mapping) or not isinstance(item.get('label'), str):
            raise MenuRecordError(f'menu item {index} must have a string label and matrix')
        try:
            keys.append(item_key(item.get('matrix')))
        except MenuRecordError as exc:
            raise MenuRecordError(f'menu item {index}: {exc}') from exc
    return keys


def _digest(keys):
    return hashlib.sha256(('[' + ','.join(keys) + ']').encode('utf-8')).hexdigest()


def menu_identity(record) -> str:
    """J1's identity: hash the compact matrix keys in sorted order."""
    return _digest(sorted(_keys(record)))


def order_digest(record) -> str:
    """Hash the same keys in the record's item order."""
    return _digest(_keys(record))


def inverse_index(record) -> list[int]:
    """Find each stored rotation's transpose in the same base-item list."""
    keys = _keys(record)
    indices = {key: index for index, key in enumerate(keys)}
    result = []
    for index, item in enumerate(record['base']):
        matrix = item['matrix']
        inverse = [[matrix[j][i] for j in range(4)] for i in range(4)]
        key = item_key(inverse)
        if key not in indices:
            raise MenuRecordError(f'menu item {index} has no inverse in the base list')
        result.append(indices[key])
    return result


def check_menu(record, *, identity=None, order=None):
    """Check the record's own identity and any expected identity and order."""
    keys = _keys(record)
    actual_identity, actual_order = _digest(sorted(keys)), _digest(keys)
    if record.get('identity') != actual_identity:
        raise MenuIdentityError('menu identity differs from the record identity field')
    if identity is not None and identity != actual_identity:
        raise MenuIdentityError('menu identity differs from the expected identity')
    if order is not None and order != actual_order:
        raise MenuOrderError('menu order digest differs from the expected order')
