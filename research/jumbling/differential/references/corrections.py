"""EH-11: the pinned 36 corrections, exact replay and a caller's drafted ν.

No function chooses, ranks or proposes corrections. `play` commits the named
corrections only after every prefix succeeds. `check_row` is observational:
it replays on a copy and leaves its input unchanged, including on a mismatch.
"""

import sys

sys.dont_write_bytecode = True

import argparse
import copy
from dataclasses import dataclass
from functools import lru_cache
import gzip
import hashlib
import json
from pathlib import Path
from time import perf_counter

ROOT = Path(__file__).resolve().parents[4]
JUMBLING = ROOT / 'research' / 'jumbling'
if str(JUMBLING) not in sys.path:
    sys.path.insert(0, str(JUMBLING))

from differential.references import nu_refs  # noqa: E402

LIBRARY = JUMBLING / 'restoration/09-general-method/data/library-merged.json.gz'
LIBRARY_SHA256 = '8a13eb6c4f4359ce4dd58a0018cf86eed3089111e11f20b7c00476f6393a661a'


class LibraryDigestError(ValueError):
    """The decompressed correction library differs from the pinned fixture."""

    def __init__(self, candidate, reference=LIBRARY_SHA256):
        self.candidate, self.reference = candidate, reference
        super().__init__(f'library sha256: candidate {candidate}, pinned {reference}')


class InadmissiblePrefix(ValueError):
    """A correction's zero-based letter index was refused; no state was changed."""

    def __init__(self, correction_id, letter_index, status):
        self.correction_id, self.letter_index, self.status = correction_id, letter_index, status
        super().__init__(f'correction {correction_id}, letter index {letter_index}: {status}')


@dataclass(frozen=True)
class Correction:
    id: int
    family: str
    length: int
    records: tuple
    letters: tuple
    row: tuple


@dataclass(frozen=True)
class Library:
    corrections: tuple
    sha256: str

    def __len__(self):
        return len(self.corrections)

    def __iter__(self):
        return iter(self.corrections)

    def __getitem__(self, cid):
        if type(cid) is not int or not 0 <= cid < len(self.corrections):
            raise ValueError(f'unknown correction id: {cid!r}')
        return self.corrections[cid]


@dataclass(frozen=True)
class RowCheck:
    correction_id: int
    before: tuple
    after: tuple
    row: tuple
    expected: tuple
    difference: dict | None

    @property
    def ok(self):
        return self.difference is None


def load_library(path=LIBRARY):
    """Check bytes before parsing; expose exact records and pinned I_a letters."""
    raw = gzip.decompress(Path(path).read_bytes())
    digest = hashlib.sha256(raw).hexdigest()
    if digest != LIBRARY_SHA256:
        raise LibraryDigestError(digest)
    document = json.loads(raw)
    if document['format'] != 'I_a-lattice-correction-library-v1' or document['menu'] != 'I_a':
        raise ValueError('the correction library has the wrong format or menu')
    macros = sorted(document['macros'], key=lambda macro: macro['id'])
    if [macro['id'] for macro in macros] != list(range(36)):
        raise ValueError('the correction library must contain ids 0 to 35 exactly once')
    census, nu = nu_refs._runtime()
    from differential.j1_state import letters_from_records

    # One matching pass shares transported frames and repeated exact matrices.
    letters = letters_from_records(census.restore.ctx, census.menu(),
                                   [record for macro in macros for record in macro['records']])
    corrections, position = [], 0
    for macro in macros:
        records, length = tuple(macro['records']), macro['length']
        if len(records) != length:
            raise ValueError(f"correction {macro['id']}: length {length}, records {len(records)}")
        row = nu.row_from_research(macro['nu'])
        nu.check_fixed(row)
        corrections.append(Correction(macro['id'], macro['family'], length, records,
                                      tuple(tuple(x) for x in letters[position:position + length]), row))
        position += length
    return Library(tuple(corrections), digest)


@lru_cache(maxsize=1)
def _library():
    return load_library()


def _clone(state):
    # J1 pose entries, exact matrices, context and menu are shared immutable data.
    result = copy.copy(state)
    result.pose_id = state.pose_id.copy()
    result._poses = list(state._poses)
    result._lookup = dict(state._lookup)
    result.journal = list(state.journal)
    result._anchor_cache = {}
    return result


@lru_cache(maxsize=None)
def _record_twist(encoded):
    census, _ = nu_refs._runtime()
    from sim.twists import Twist
    return Twist.from_record(census.restore.ctx, json.loads(encoded))


def _selected(correction_or_ids):
    if isinstance(correction_or_ids, Correction):
        return (correction_or_ids,)
    ids = [correction_or_ids] if type(correction_or_ids) is int else list(correction_or_ids)
    library = _library()
    return tuple(library[cid] for cid in ids)


def _literal_play(state, correction):
    for index, record in enumerate(correction.records):
        twist = _record_twist(json.dumps(record, sort_keys=True, separators=(',', ':')))
        outcome = state.apply(twist)
        if not outcome.applied:
            raise InadmissiblePrefix(correction.id, index, outcome.status)


@lru_cache(maxsize=36)
def _lattice_action(cid, encoded_records):
    """A full J1 prefix witness and its exact final K+ action, from solved.

    Stage 03 section 3.2 / manual 04 section 4.3: every valid lattice state
    fills the same piece regions as solved. Thus each prefix is admissible
    from it exactly when that prefix is admissible from solved. Caching this
    witness avoids repeating classification for thousands of differently
    labelled copies of the same shapes; no ν row is used to execute it.
    """
    census, _ = nu_refs._runtime()
    from sim.state import State

    state = State(census.restore.ctx, menu=census.menu())
    for index, encoded in enumerate(encoded_records):
        outcome = state.apply(_record_twist(encoded))
        if not outcome.applied:
            raise InadmissiblePrefix(cid, index, outcome.status)
    if not state.is_lattice():
        raise ValueError(f'correction {cid}: the complete J1 witness does not end on the lattice')
    action = state._kpose()[state.pose_id]
    action.flags.writeable = False
    return action, tuple(state.journal)


def _transport_action(state, action, journal):
    """Apply the witnessed left action to every actual piece, preserving its frame.

    J1's lattice_stickers uses this same region lookup: q is the region now
    occupied by source piece p. If its pose is K[p], its final pose is
    action[q] o K[p], not K[p] o action[q]. All pieces, including centres and
    collateral pieces, participate. Only J1's exact K+ arithmetic is used.
    """
    import numpy as np

    kp, regions = state.ctx.kplus, state.ctx.regions
    before = state._kpose()[state.pose_id]
    destination = regions.piece_of[regions.orbit_of, kp.compose_vec(before, regions.transport)]
    if not np.array_equal(np.sort(destination), np.arange(len(destination))):
        raise ValueError('the lattice piece-region map is not a bijection')
    after = kp.compose_vec(action[destination], before)
    unique, inverse = np.unique(after, return_inverse=True)
    indices = np.asarray([state._intern_k(int(k)) for k in unique], np.int32)
    state.pose_id = indices[inverse]
    state._compact()
    state.journal.extend(copy.deepcopy(journal))
    state._anchor_cache = {}


def play(state, correction_or_ids, *, literal=False):
    """Play only the caller's corrections, in order; roll back the whole call on refusal.

    On a lattice state each correction's exact action is transported from its replay from solved
    (`_lattice_action`); with `literal=True` every twist is replayed on the state itself."""
    census, nu = nu_refs._runtime()
    corrections = _selected(correction_or_ids)  # validate all ids before any twist
    trial = _clone(state)
    for correction in corrections:
        if (not literal and trial.is_lattice() and trial.ctx is census.restore.ctx
                and (trial.menu is None or trial.menu.identity == census.menu().identity)):
            encoded = tuple(json.dumps(record, sort_keys=True, separators=(',', ':'))
                            for record in correction.records)
            action, journal = _lattice_action(correction.id, encoded)
            _transport_action(trial, action, journal)
        else:
            _literal_play(trial, correction)
    value = nu.nu(trial)
    nu.check_fixed(value)
    state.__dict__.update(trial.__dict__)
    return value


def check_row(state, cid, *, literal=False):
    """Compare ν after a correction with ν before + its row, without changing `state`."""
    _, nu = nu_refs._runtime()
    before = nu.nu(state)  # typed NuUndefined before replay if the input is off lattice
    nu.check_fixed(before)
    correction = _library()[cid]
    after = play(_clone(state), correction, literal=literal)
    expected = nu.add(before, correction.row)
    difference = nu_refs._difference(after, expected, 'library')
    if difference:
        difference = dict(correction_id=cid, **difference)
    return RowCheck(cid, before, after, correction.row, expected, difference)


def drafted_nu(value, ids):
    """Add only the fixed rows of the supplied ids; no state or twist is constructed."""
    _, nu = nu_refs._runtime()
    corrections = _selected(ids)
    result = tuple(value)
    nu.first_difference(result, nu.ZERO)  # enforce the 45-coordinate shape, including empty sets
    nu.check_fixed(result)
    for correction in corrections:
        nu.check_fixed(correction.row)
        result = nu.add(result, correction.row)
    return result


def run_checks(start='solved', ids=None, literal=False):
    census, nu = nu_refs._runtime()
    from sim.state import State

    library = _library()
    selected = list(range(len(library))) if ids is None else list(ids)
    for cid in selected:
        library[cid]
    state = (State(census.restore.ctx, menu=census.menu()) if start == 'solved'
             else nu_refs.table_state(nu_refs.TABLES[start])[0])
    rows = []
    for cid in selected:
        started = perf_counter()
        result = check_row(state, cid, literal=literal)
        rows.append({'id': cid, 'length': library[cid].length, 'seconds': perf_counter() - started,
                     'nu_before': nu.nonzero(result.before), 'nu_after': nu.nonzero(result.after),
                     'row': nu.nonzero(result.row), 'ok': result.ok, 'difference': result.difference})
    summary = {'from': start, 'literal': literal, 'requested': len(selected), 'checked': len(rows),
               'passed': sum(row['ok'] for row in rows), 'failed': sum(not row['ok'] for row in rows),
               'letters': sum(row['length'] for row in rows), 'library_sha256': library.sha256,
               'ok': len(rows) == len(selected) and all(row['ok'] for row in rows), **nu_refs.run_metadata()}
    return rows, summary


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True)
    parser.add_argument('--from', dest='start', choices=('solved', 'E3', 'I1', 'I2'), default='solved')
    parser.add_argument('--ids', nargs='+', type=int)
    parser.add_argument('--literal', action='store_true',
                        help='replay every twist on the start state instead of transporting the action from solved')
    args = parser.parse_args(argv)
    stem = f'corrections-{args.start}' + ('-literal' if args.literal else '')
    names = (f'{stem}.jsonl', f'{stem}-summary.json')
    try:
        directory = nu_refs.output_directory(args.out, names)
    except ValueError as error:
        parser.error(str(error))
    rows, summary = run_checks(args.start, args.ids, args.literal)
    directory.mkdir(parents=True, exist_ok=True)
    with (directory / names[0]).open('w', encoding='utf-8') as target:
        for row in rows:
            target.write(json.dumps(row) + '\n')
    (directory / names[1]).write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
    return 0 if summary['ok'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
