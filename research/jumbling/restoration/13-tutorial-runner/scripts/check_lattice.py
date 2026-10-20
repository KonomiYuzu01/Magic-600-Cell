"""Literal lattice steps of runner-run/3 records, checker side (headless research).

Stub until packet P5 is integrated. ``read_ia16`` is complete: check_run.py reads
every ``ia16`` word through it. ``replay_lattice_step`` plays a lattice step's word
on the exact simulator twist by twist (correct, but far too slow for a real lattice
word) and reports ``unchecked_contract``: the step's ``lattice_literal`` contract and
its ``labels_identity`` claim are not checked until P5 (plan runner-run/3, packet P5).

Interface used by check_run.py:
- ``read_ia16(path, sha256=None)`` returns the letters ``[[grip, item], ...]`` of an
  ia16 word: gzip with mtime 0 of little-endian uint16 codes ``60 * grip + item``, all
  below 36,000. It raises ``ValueError`` for a malformed file and ``Sha256Mismatch``
  when ``sha256`` is given and differs from the sha256 of the uncompressed bytes.
- ``replay_lattice_step(state, row, path, menu)`` plays the lattice step ``row`` whose
  word is the file ``path`` on ``state``, in place, and returns its failures, a list of
  names. check_run.py then checks the state's digest and off count against the row's
  ``after``. A step whose ``labels_identity`` it does not confirm fails as
  ``labels_identity``; until P5 that claim is ``unchecked_contract``.

Like check_run.py, this module shares only the definition of the puzzle: the simulator
and the I_a menu, no runner, rule, gate or player code (plan section 0).
"""

import gzip
import hashlib
from pathlib import Path
import sys
import zlib

sys.dont_write_bytecode = True
J = Path(__file__).resolve().parents[3]
for _path in (J / 'fixtures', J, J / 'theory'):
    if str(_path) not in sys.path:
        sys.path.insert(0, str(_path))

import numpy as np  # noqa: E402
import restore  # noqa: E402
import sim  # noqa: E402

GRIPS, ITEMS = 600, 60
UNCHECKED = 'unchecked_contract'


class Sha256Mismatch(ValueError):
    pass


def read_ia16(path, sha256=None):
    raw = Path(path).read_bytes()
    # A fixed header (mtime 0) keeps equal words byte-identical.
    if raw[:2] != b'\x1f\x8b' or raw[4:8] != bytes(4):
        raise ValueError('an ia16 word is gzip with mtime 0')
    data = gzip.decompress(raw)
    if len(data) % 2:
        raise ValueError('an ia16 word has whole uint16 codes')
    if sha256 is not None and hashlib.sha256(data).hexdigest() != sha256:
        raise Sha256Mismatch('ia16 word sha256 mismatch')
    codes = np.frombuffer(data, '<u2').astype(np.int64)
    if codes.size and int(codes.max()) >= GRIPS * ITEMS:
        raise ValueError('ia16 code out of range')
    grips, items = np.divmod(codes, ITEMS)
    return [[g, i] for g, i in zip(grips.tolist(), items.tolist())]


def replay_lattice_step(state, row, path, menu):
    if not isinstance(row.get('word_sha256'), str):
        return ['word_sha256']
    try:
        word = read_ia16(path, row['word_sha256'])
    except Sha256Mismatch:
        return ['word_sha256']
    except (OSError, EOFError, ValueError, zlib.error):
        return ['word_file']
    fails = []
    if type(row.get('letters')) is not int or row['letters'] != len(word):
        fails.append('letters')
    for cap, item in word:
        try:
            outcome = state.apply(sim.Twist(restore.ctx, cap, restore.element(menu, cap, item),
                                            'menu', {'menu': menu.name}), journal=False)
            admitted = outcome.applied and not outcome.uncertain
        except Exception:  # a simulator error cannot certify a letter
            admitted = False
        if not admitted:
            fails.append('admissible')
            break
    return fails + [UNCHECKED]
