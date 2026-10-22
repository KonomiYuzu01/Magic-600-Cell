"""Compare engine observations after each record, stopping at the first mismatch."""

from collections.abc import Iterable, Mapping, Sequence
from copy import deepcopy
from dataclasses import asdict, dataclass
import hashlib
import json
from os import PathLike
from pathlib import Path
from typing import Protocol

from .menu_order import MenuRecordError, check_menu, order_digest


STATUSES = ('admissible', 'blocked', 'invalid', 'uncertain')
EMPTY_MOVED_SET = hashlib.sha256(b'').hexdigest()
_HEX = frozenset('0123456789abcdef')


class HarnessError(Exception):
    """Base error for the differential harness."""


class HarnessInputError(HarnessError):
    """An input document or comparison argument is malformed."""


class ReferenceContractError(HarnessError):
    """The reference engine violated the observation contract."""


def moved_set_hash(ids: Iterable[int]) -> str:
    """Hash the sorted set of piece ids as little-endian uint32 bytes."""
    members = set()
    for piece in ids:
        if type(piece) is not int or not 0 <= piece < 2**32:
            raise HarnessInputError('moved piece ids must be uint32 integers, excluding booleans')
        members.add(piece)
    h = hashlib.sha256()
    for piece in sorted(members):
        h.update(piece.to_bytes(4, 'little'))
    return h.hexdigest()


@dataclass(frozen=True)
class StepResult:
    status: str
    moved: int
    moved_set: str
    digest: str
    lattice: str


class Engine(Protocol):
    name: str

    def start(self, menu: Mapping | None) -> StepResult: ...

    def apply(self, step: Mapping) -> StepResult: ...

    def survey(self, grips: Sequence[int]) -> Mapping[int, str]: ...


@dataclass(frozen=True)
class Steps:
    steps: tuple[Mapping, ...]
    menu: Mapping | None = None
    model_identity: Mapping | None = None
    menu_identity: str | None = None
    contract_revision: str | None = None
    menu_order_digest: str | None = None


@dataclass(frozen=True)
class Mismatch:
    step: int
    step_index_in_input: int | None
    field: str
    reference: object
    candidate: object
    detail: str


@dataclass(frozen=True)
class Report:
    passed: bool
    engines: tuple[str, str]
    steps_compared: int
    surveys_compared: int
    first_mismatch: Mismatch | None

    def as_dict(self):
        return {'passed': self.passed, 'engines': list(self.engines),
                'steps_compared': self.steps_compared, 'surveys_compared': self.surveys_compared,
                'first_mismatch': (None if self.first_mismatch is None
                                   else asdict(self.first_mismatch))}


def _grip(value, where):
    if type(value) is not int or not 0 <= value < 600:
        raise HarnessInputError(f'{where}: grip must be an integer in 0..599')
    return value


def _menu(record):
    try:
        check_menu(record)
    except MenuRecordError as exc:
        raise HarnessInputError(f'menu: {exc}') from exc
    return deepcopy(record)


def _is_hash(value):
    return isinstance(value, str) and len(value) == 64 and all(c in _HEX for c in value)


def load_steps(source, menu=None) -> Steps:
    """Read a journal, W-J fixture or solved letter word; never load an engine."""
    if isinstance(source, (str, PathLike)):
        try:
            source = json.loads(Path(source).read_text(encoding='utf-8'))
        except (OSError, ValueError) as exc:
            raise HarnessInputError(f'cannot read input document: {exc}') from exc
    if not isinstance(source, Mapping):
        raise HarnessInputError('input document must be a mapping')

    fmt = source.get('format')
    if fmt == 'magic600-jumbling-wj/1':
        fixture_menu = _menu(source.get('menu'))
        if menu is not None:
            supplied = _menu(menu)
            try:
                check_menu(supplied, identity=fixture_menu['identity'], order=order_digest(fixture_menu))
            except MenuRecordError as exc:
                raise HarnessInputError(f'fixture and supplied menu differ: {exc}') from exc
        journal = source.get('journal')
        if not isinstance(journal, Mapping) or journal.get('format') != 'jumbling-journal-1':
            raise HarnessInputError('W-J fixture must contain a jumbling-journal-1 document')
        return load_steps(journal, fixture_menu)

    selected_menu = None if menu is None else _menu(menu)
    if fmt == 'jumbling-journal-1':
        records = source.get('records')
        if not isinstance(records, list):
            raise HarnessInputError('journal records must be a list')
        normalized = []
        for index, record in enumerate(records):
            if not isinstance(record, Mapping):
                raise HarnessInputError(f'record {index} must be a mapping')
            _grip(record.get('grip'), f'record {index}')
            normalized.append({'kind': 'record', 'record': deepcopy(record)})
        model = source.get('model_identity')
        if model is not None and (not isinstance(model, Mapping) or any(
                not isinstance(k, str) or not _is_hash(v) for k, v in model.items())):
            raise HarnessInputError('model_identity must map asset names to SHA-256 hashes')
        identity = source.get('menu_identity')
        if identity is not None and not _is_hash(identity):
            raise HarnessInputError('menu_identity must be a SHA-256 hash or null')
        revision = source.get('contract_revision')
        if revision is not None and not isinstance(revision, str):
            raise HarnessInputError('contract_revision must be a string')
        if selected_menu is not None:
            if 'menu_identity' in source and identity != selected_menu['identity']:
                raise HarnessInputError('journal menu_identity differs from the supplied menu')
            identity = selected_menu['identity']
        return Steps(tuple(normalized), selected_menu, deepcopy(model), identity, revision,
                     None if selected_menu is None else order_digest(selected_menu))

    if fmt is None and source.get('from') == 'solved' and 'word' in source:
        if selected_menu is None:
            raise HarnessInputError('a letter word requires a menu record')
        word = source['word']
        if not isinstance(word, list):
            raise HarnessInputError('word must be a list of [grip, item] letters')
        normalized = []
        for index, letter in enumerate(word):
            if not isinstance(letter, list) or len(letter) != 2:
                raise HarnessInputError(f'letter {index} must be a [grip, item] list')
            grip, item = letter
            _grip(grip, f'letter {index}')
            if type(item) is not int or not 0 <= item < len(selected_menu['base']):
                raise HarnessInputError(f'letter {index}: item is outside the menu base')
            normalized.append({'kind': 'letter', 'grip': grip, 'item': item})
        return Steps(tuple(normalized), selected_menu, menu_identity=selected_menu['identity'],
                     menu_order_digest=order_digest(selected_menu))
    raise HarnessInputError('unknown input format')


def _result_error(result, step):
    if not isinstance(result, StepResult):
        return 'engine must return a StepResult'
    allowed = ('start',) if step == 0 else STATUSES
    if not isinstance(result.status, str) or result.status not in allowed:
        return f'status must be one of {allowed}'
    if type(result.moved) is not int or result.moved < 0:
        return 'moved must be a non-negative integer, excluding booleans'
    for field in ('moved_set', 'digest', 'lattice'):
        if not _is_hash(getattr(result, field)):
            return f'{field} must be a 64-character lowercase hex SHA-256 hash'
    return None


def _refusal_error(result, previous):
    if previous is None or result.status == 'admissible':
        return None
    changed = [field for field, expected in (
        ('moved', 0), ('moved_set', EMPTY_MOVED_SET),
        ('digest', previous.digest), ('lattice', previous.lattice))
        if getattr(result, field) != expected]
    return None if not changed else 'refused step changed ' + ', '.join(changed)


def _survey_error(result, grips):
    if not isinstance(result, Mapping):
        return 'survey must return a mapping of grip ids to letters'
    if any(type(g) is not int for g in result) or set(result) != set(grips):
        return 'survey must return exactly the requested integer grip ids'
    for grip in grips:
        if not isinstance(result[grip], str) or result[grip] not in ('a', 'b', 'u'):
            return f'survey grip {grip} must be a letter a, b or u'
    return None


def compare(reference: Engine, candidate: Engine, steps: Steps, *, survey=None, grips=None) -> Report:
    """Compare observations, validating each side and each refusal independently."""
    if not isinstance(steps, Steps):
        raise HarnessInputError('compare requires Steps returned by load_steps')
    if survey is None:
        surveyed = set()
    elif survey == 'all':
        surveyed = set(range(len(steps.steps) + 1))
    elif isinstance(survey, (set, frozenset)):
        surveyed = set(survey)
        if any(type(i) is not int or not 0 <= i <= len(steps.steps) for i in surveyed):
            raise HarnessInputError('survey indices must name steps in 0..len(steps)')
    else:
        raise HarnessInputError("survey must be None, 'all' or a set of step indices")
    try:
        requested = tuple(sorted({_grip(g, 'survey') for g in (range(600) if grips is None else grips)}))
    except TypeError as exc:
        raise HarnessInputError('grips must be a sequence of grip ids') from exc
    names = (reference.name, candidate.name)
    if not all(isinstance(name, str) for name in names):
        raise HarnessInputError('engine names must be strings')
    surveys = 0
    previous_reference = previous_candidate = None

    def failed(step, field, ref_value, candidate_value, detail):
        mismatch = Mismatch(step, None if step == 0 else step - 1, field,
                            ref_value, candidate_value, detail)
        return Report(False, names, step + 1, surveys, mismatch)

    for index in range(len(steps.steps) + 1):
        if index == 0:
            ref = reference.start(deepcopy(steps.menu))
            cand = candidate.start(deepcopy(steps.menu))
        else:
            step = steps.steps[index - 1]
            ref = reference.apply(deepcopy(step))
            cand = candidate.apply(deepcopy(step))
        reason = _result_error(ref, index) or _refusal_error(ref, previous_reference)
        if reason is not None:
            raise ReferenceContractError(f'{names[0]}, step {index}: {reason}')
        reason = _result_error(cand, index)
        if reason is not None:
            return failed(index, 'contract', 'valid StepResult', reason, reason)
        reason = _refusal_error(cand, previous_candidate)
        if reason is not None:
            return failed(index, 'refusal_changed_state', asdict(ref), asdict(cand), reason)
        for field in ('status', 'moved', 'moved_set', 'digest', 'lattice'):
            ref_value, cand_value = getattr(ref, field), getattr(cand, field)
            if ref_value != cand_value:
                return failed(index, field, ref_value, cand_value,
                              f'{field} differs at step {index}')
        if index in surveyed:
            ref_grips = reference.survey(requested)
            cand_grips = candidate.survey(requested)
            surveys += 1
            reason = _survey_error(ref_grips, requested)
            if reason is not None:
                raise ReferenceContractError(f'{names[0]}, step {index}: {reason}')
            reason = _survey_error(cand_grips, requested)
            if reason is not None:
                return failed(index, 'contract', 'valid grip-letter mapping', reason, reason)
            for grip in requested:
                if ref_grips[grip] != cand_grips[grip]:
                    return failed(index, 'grips', ref_grips[grip], cand_grips[grip],
                                  f'grip {grip} differs at step {index}')
        previous_reference, previous_candidate = ref, cand
    return Report(True, names, len(steps.steps) + 1, surveys, None)
