"""ν of a labelled lattice configuration and the label checkpoint of amendment A5.

ν(X) is the invariant of the labelled state π(X) (state contract, amendments A3 and A5): the 35 sign
bits b0 to b34, the 8 frame bits f0, f3, f6, f8, f11, f22, f25 and f32, and the two values modulo 5
C5_17 and C5_28, in that order. The research reference `nucoords.py` of stage 03 computes it from
J1's labelled stickers (`State.lattice_stickers`). ν is defined on lattice configurations only, and
the C5 values are read in the generator r0 that `nucoords.py` picks (the C5 convention of A5).

Reachability (a journal from solved, or from a stored exit state reached so, that reaches the
configuration) is the caller's check. A refusal names the fact and the kind of action, never a
correction or a twist.
"""

import sys

sys.dont_write_bytecode = True

import hashlib  # noqa: E402
from functools import lru_cache  # noqa: E402
from pathlib import Path  # noqa: E402

import numpy as np  # noqa: E402

ROOT = Path(__file__).resolve().parents[4]
JUMBLING = ROOT / 'research' / 'jumbling'
STAGE03 = JUMBLING / 'restoration' / '03-retained-group-and-lattice-restorer'
for _path in (JUMBLING, STAGE03 / 'scripts'):
    if str(_path) not in sys.path:
        sys.path.insert(0, str(_path))

from nucoords import C2F, C5F, NuCoords, vec  # noqa: E402

NAMES = tuple([f'b{o}' for o in range(35)] + [f'f{o}' for o in C2F] + [f'C5_{o}' for o in C5F])
MODULI = (2,) * 43 + (5,) * 2
ZERO = (0,) * 45
# Stage 04, Theorems A and B5: 0 on every reachable lattice configuration, under the I_a menu only.
I_A_FIXED = ('b0', 'b1', 'b2', 'b3', 'b5', 'b6', 'f0', 'f3', 'f6')
C5_CONVENTION = 'nucoords-r0: the least order-5 frame permutation of the third buffer'
CONTRACT_REVISION = 'state-contract 2026-10-22 A1-A5'
CERTIFICATE_PROGRAM = STAGE03 / 'certificates' / 'certificates.json.gz'


class NuUndefined(ValueError):
    """ν and the labels π are defined on lattice configurations only."""


class NuConsistencyError(AssertionError):
    """A coordinate that is 0 on every reachable I_a lattice configuration is not 0."""


class LabelCheckpointRefused(ValueError):
    """The configuration is not a label checkpoint; `fact` names why."""

    def __init__(self, fact, message):
        super().__init__(message)
        self.fact = fact


@lru_cache(maxsize=1)
def coords():
    return NuCoords(ROOT)


def labels(state):
    """π(X) as int64: labels[slot] is the home slot of the sticker now in slot."""
    if not state.is_lattice():
        raise NuUndefined(f'{state.off_lattice_count()} pieces are off the lattice')
    stickers = state.lattice_stickers()
    if not bool(np.all(stickers['frame_agrees'])):
        raise AssertionError('a sticker frame disagrees with its slot layout (amendment A3)')
    return np.asarray(stickers['labels'], dtype=np.int64)


def labels_digest(values):
    """SHA-256 of the labels as little-endian int64, as the research restorers record it."""
    return hashlib.sha256(np.asarray(values, dtype='<i8').tobytes()).hexdigest()


def nu_of_labels(values):
    b2, b5 = vec(coords().nu(np.asarray(values, dtype=np.int64)))
    return tuple(int(x) for x in b2 + b5)


def nu(state):
    return nu_of_labels(labels(state))


def add(value, row):
    """ν after a correction is ν before it plus the correction's row (card 1, F1)."""
    return tuple((a + b) % m for a, b, m in zip(value, row, MODULI, strict=True))


def row_from_research(entries):
    """A research ν record ({orbit: [sign, frame]}, nonzero orbits only) as 45 coordinates."""
    b2, b5 = vec({int(orbit): (int(sign), int(frame)) for orbit, (sign, frame) in entries.items()})
    return tuple(int(x) for x in b2 + b5)


def nonzero(value):
    return {NAMES[i]: v for i, v in enumerate(value) if v}


def first_difference(candidate, reference):
    """None when equal, else (name, candidate value, reference value) of the first difference."""
    if len(candidate) != 45 or len(reference) != 45:
        raise ValueError('ν has 45 coordinates')
    for name, a, b in zip(NAMES, candidate, reference):
        if a != b:
            return name, a, b
    return None


def check_fixed(value):
    """Under I_a the nine fixed coordinates are 0; a nonzero one is a consistency error, never a state."""
    bad = [name for name in I_A_FIXED if value[NAMES.index(name)]]
    if bad:
        raise NuConsistencyError('nonzero fixed ν coordinate under I_a: ' + ', '.join(bad))


def centres_home(state):
    """Every centre in its own chamber: the inline test of J1's `State.checkpoint`."""
    if not state.is_lattice():
        raise NuUndefined(f'{state.off_lattice_count()} pieces are off the lattice')
    data, kp = state.ctx.data, state.ctx.kplus
    k = state._kpose()[state.pose_id[data.centre_pieces]]
    return not bool(np.any(kp.perms[k, data.centre_poles] != data.centre_poles))


@lru_cache(maxsize=1)
def certificate_program_sha256():
    return hashlib.sha256(CERTIFICATE_PROGRAM.read_bytes()).hexdigest()


def label_certificate(state, *, i_a=True):
    """The certificate of a label checkpoint (A5), in the fields of the store prototype.

    Checks the lattice test, the centres, the frame agreement and ν = 0 under the C5 convention;
    the caller has shown the configuration reachable. With `i_a`, the nine fixed coordinates are
    checked first (stage 04 bounds them for I_a only)."""
    if not state.is_lattice():
        raise LabelCheckpointRefused(
            'off_lattice', 'pieces are off the lattice; twists that bring them back are needed')
    if not centres_home(state):
        raise LabelCheckpointRefused('centre_displaced', 'a centre is outside its own chamber')
    values = labels(state)
    value = nu_of_labels(values)
    if i_a:
        check_fixed(value)
    if any(value):
        raise LabelCheckpointRefused('nu_nonzero', 'ν is not 0; corrections are needed')
    return {'pose_digest': state.digest(), 'labels_digest': labels_digest(values), 'nu': list(value),
            'c5_convention': C5_CONVENTION, 'certificate_program_sha256': certificate_program_sha256(),
            'contract_revision': CONTRACT_REVISION}
