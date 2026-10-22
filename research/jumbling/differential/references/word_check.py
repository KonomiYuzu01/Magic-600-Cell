"""Check a caller's word at one exact J1 revision; never choose a word.

python -B research/jumbling/differential/references/word_check.py --out DIR \
    [--extract DIR] [--cases u1 b20 b06-14 b14 b03 r22b34]

Evidence is source/fixture replay. Timings are research-code cloud timings,
not product-engine or Windows/DirectX performance evidence. Extracted inputs
must preserve repository-relative paths beneath --extract. Tests never read
the draft branch. Unavailable or partial cases never count as comparisons.
"""

import sys

sys.dont_write_bytecode = True

import argparse
from collections.abc import Mapping
import copy
from dataclasses import asdict, dataclass
import gzip
import hashlib
import json
from pathlib import Path
from time import perf_counter

ROOT = Path(__file__).resolve().parents[4]
JUMBLING = ROOT / 'research/jumbling'
LOW = 'research/jumbling/restoration/12-low-height/'
COUNT_KEYS = ('n_0', 'n_2', 'n_4', 'n_6', 'n_above_6')


def _research():
    if str(JUMBLING) not in sys.path:
        sys.path.insert(0, str(JUMBLING))
    # Must precede every theory/restoration import, including optional caches.
    from differential.leads import census
    return census


def copy_state(state):
    """Copy live state data, sharing only context and immutable exact poses."""
    _research()
    state = getattr(state, 'st', state)
    trial = copy.copy(state)
    trial.pose_id = state.pose_id.copy()
    trial._poses, trial._lookup = list(state._poses), dict(state._lookup)
    trial.journal = copy.deepcopy(state.journal)
    trial._anchor_cache = {}
    return trial


def _menu(state):
    return state.menu if state.menu is not None else _research().menu()


def _letters(letters, menu):
    letters = list(letters)
    for index, pair in enumerate(letters):
        if (not isinstance(pair, (list, tuple)) or len(pair) != 2
                or type(pair[0]) is not int or not 0 <= pair[0] < 600
                or type(pair[1]) is not int or not 0 <= pair[1] < len(menu)):
            raise ValueError(f'letter {index}: expected integer [cap, menu item]')
    return [[c, x] for c, x in letters]


def word_digest(letters):
    """SHA-256 of chronological pairs as compact canonical JSON (no spaces)."""
    raw = json.dumps([list(pair) for pair in letters], separators=(',', ':')).encode('ascii')
    return hashlib.sha256(raw).hexdigest()


def _heights(state):
    import numpy as np
    import trees

    result = dict.fromkeys(COUNT_KEYS, 0)
    for pid, count in zip(*np.unique(state.pose_id, return_counts=True)):
        pose = state._poses[int(pid)]
        height = 0 if pose.kidx >= 0 else trees.dist(trees.factors(state._pose_matrix(int(pid)))[0])
        if height not in (0, 2, 4, 6) and height <= 6:
            raise ValueError(f'pose has unexpected I_a left height {height}')
        result[f'n_{height}' if height <= 6 else 'n_above_6'] += int(count)
    return result


def _u(state, cap):
    if cap is None:
        return None
    from differential.straddlers import straddler_set

    result = straddler_set(state, cap)
    if result['uncertain_pieces']:
        raise ValueError(f'cap {cap}: straddler set is uncertain')
    return len(result['pieces'])


@dataclass(frozen=True)
class WordCheck:
    start_digest: str
    end_digest: str
    letters: int
    word_digest: str
    unit: tuple[int, ...]
    unit_size: int
    unit_on_lattice: int
    lattice_pieces_lifted_off: int
    collateral_ids: tuple[int, ...]
    others: int
    others_exact: int
    others_landed: int
    others_changed_ids: tuple[int, ...]
    off_before: int
    off_after: int
    off_min: int
    off_max: int
    off_pieces_landed: int
    pose_changed: int
    pose_changed_ids: tuple[int, ...]
    delta_N: int
    heights_before: dict
    heights_after: dict
    delta_n_h: dict
    cap: int | None
    u_before: int | None
    u_after: int | None
    delta_u: int | None
    admissible: bool
    failing_letter_index: int | None
    failing_status: str | None
    applied_letters: int
    step_flag: bool

    @property
    def input_digest(self):
        return self.start_digest

    @property
    def collateral(self):
        return self.lattice_pieces_lifted_off

    @property
    def inadmissible(self):
        return None if self.admissible else dict(prefix_index=self.failing_letter_index, status=self.failing_status)

    @property
    def collateral_sample(self):
        return list(self.collateral_ids[:200])

    @property
    def changed_sample(self):
        return list(self.pose_changed_ids[:200])


def _check(state, letters, unit, cap, journal=False):
    _research()
    import numpy as np
    import rim
    from differential.j1_state import twists_from_letters

    start = copy_state(state)
    menu = _menu(start)
    letters = _letters(letters, menu)
    unit = list(unit)
    if any(type(p) is not int or not 0 <= p < len(start.pose_id) for p in unit):
        raise ValueError('unit must contain integer piece ids')
    unit = tuple(sorted(set(unit)))
    if cap is not None and (type(cap) is not int or not 0 <= cap < 600):
        raise ValueError('cap must be an integer in 0..599')
    before = start.lattice_flags()
    off0 = np.flatnonzero(~before).tolist()
    poses0 = {p: start._poses[int(start.pose_id[p])].key for p in off0}
    heights_before = _heights(start)
    N_before = rim.Domains(start).total_misalignment()
    u_before = _u(start, cap)
    end = copy_state(start)
    counts = [len(off0)]
    failed, status, applied = None, None, 0
    for index, twist in enumerate(twists_from_letters(menu, letters)):
        outcome = end.apply(twist, journal=journal)
        if not outcome.applied:
            failed, status = index, outcome.status
            break
        applied += 1
        counts.append(end.off_lattice_count())
    after = end.lattice_flags()
    other = sorted(set(off0) - set(unit))
    changed = tuple(p for p in off0 if not after[p] and end._poses[int(end.pose_id[p])].key != poses0[p])
    changed_set = set(changed)
    others_changed = tuple(p for p in other if p in changed_set)
    exact = sum(not after[p] and p not in changed_set for p in other)
    landed = sum(bool(after[p]) for p in other)
    on_lattice = sum(bool(after[p]) for p in unit)
    collateral = tuple(np.flatnonzero(before & ~after).tolist())
    heights_after, u_after = _heights(end), _u(end, cap)
    check = WordCheck(
        start_digest=start.digest(), end_digest=end.digest(), letters=len(letters), word_digest=word_digest(letters),
        unit=unit, unit_size=len(unit), unit_on_lattice=on_lattice,
        lattice_pieces_lifted_off=len(collateral), collateral_ids=collateral,
        others=len(other), others_exact=int(exact), others_landed=landed, others_changed_ids=others_changed,
        off_before=len(off0), off_after=int((~after).sum()), off_min=min(counts), off_max=max(counts),
        off_pieces_landed=int((~before & after).sum()), pose_changed=len(changed), pose_changed_ids=changed,
        delta_N=rim.Domains(end).total_misalignment() - N_before,
        heights_before=heights_before, heights_after=heights_after,
        delta_n_h={k: heights_after[k] - heights_before[k] for k in COUNT_KEYS},
        cap=cap, u_before=u_before, u_after=u_after, delta_u=None if cap is None else u_after - u_before,
        admissible=failed is None, failing_letter_index=failed, failing_status=status, applied_letters=applied,
        step_flag=failed is None and on_lattice == len(unit) and not collateral and exact + landed == len(other))
    return check, end


def word_check(state, letters, unit, *, cap=None) -> WordCheck:
    """Replay on a copy. A refused prefix describes the successful prefix only."""
    return _check(state, letters, unit, cap)[0]


def release_fields(S, c, x, *, cap=None):
    """X5's one-item hypothetical check plus exact end digest and named-cap u.

    This path does not use word_check or total_misalignment: agreement with a
    one-letter word independently tests the research own-cut delta-N identity.
    """
    _research()
    from differential.leads import x5_release
    from differential.j1_state import twists_from_letters
    import seams

    state = copy_state(S)
    restorer = seams.SeamRestorer(state, _menu(state))
    result = x5_release.release_check(restorer, c, x)
    before = _u(state, cap)
    outcome = state.apply(twists_from_letters(restorer.M, [[c, x]])[0], journal=False)
    if not outcome.applied:
        raise BlockedWordError(0, outcome.status)
    after = _u(state, cap)
    result.update(start_digest=result['input_digest'], end_digest=state.digest(),
                  word_digest=word_digest([[c, x]]), cap_named=cap, u_before=before, u_after=after,
                  delta_u=None if cap is None else after - before)
    return result


class StalePreviewError(ValueError):
    def __init__(self, field, expected, actual):
        self.field, self.expected, self.actual = field, expected, actual
        super().__init__(f'{field}: preview {expected!r}, current {actual!r}')


class BlockedWordError(ValueError):
    def __init__(self, index, status):
        self.index, self.status = index, status
        super().__init__(f'letter {index}: {status}; no word committed')


@dataclass(frozen=True)
class Preview:
    check: WordCheck
    end_state: object
    end_digest: str
    word_digest: str


def preview(state, letters, unit=(), *, cap=None) -> Preview:
    """The word check plus the end state, whose journal records the word as a direct application would."""
    check, end = _check(state, letters, unit, cap, journal=True)
    return Preview(check, end, check.end_digest, check.word_digest)


def commit(state, preview, letters):
    """Return a copy of the previewed end; never replay or mutate the input.

    Failure of step_flag is an observation, not a refusal. The caller has
    supplied no protection set. Prefix refusal and stale input are refusals.
    """
    state = getattr(state, 'st', state)
    actual = state.digest()
    if actual != preview.check.start_digest:
        raise StalePreviewError('start_digest', preview.check.start_digest, actual)
    actual = word_digest(_letters(letters, _menu(state)))
    if actual != preview.word_digest:
        raise StalePreviewError('word_digest', preview.word_digest, actual)
    if not preview.check.admissible:
        raise BlockedWordError(preview.check.failing_letter_index, preview.check.failing_status)
    actual = preview.end_state.digest()
    if actual != preview.end_digest or actual != preview.check.end_digest:
        raise StalePreviewError('end_digest', preview.check.end_digest, actual)
    return copy_state(preview.end_state)


# Reference field -> WordCheck field. Unit number and eb4's complementary
# frame part (free) are not functions of the caller's piece set, so are not
# shared fields. Neither research row supplies delta_N, delta_n_h or delta_u.
EB4_MAPPING = dict(letters='letters', unit_size='unit_size', unit_on_lattice='unit_on_lattice',
                   collateral='lattice_pieces_lifted_off', others='others', others_exact='others_exact',
                   others_landed='others_landed', off_after='off_after', strict_prime='step_flag')
STEP_MAPPING = dict(start_digest='start_digest', end_digest='end_digest', **EB4_MAPPING,
                    off_before='off_before')
REPLAY_MAPPING = dict(twists='applied_letters', end_digest='end_digest', off_min='off_min', off_max='off_max',
                      off_end='off_after', restored='off_pieces_landed', collateral='lattice_pieces_lifted_off',
                      collateral_ids='collateral_sample', pose_changed='pose_changed',
                      pose_changed_ids='changed_sample', inadmissible='inadmissible')


@dataclass(frozen=True)
class Comparison:
    passed: bool
    field: str | None = None
    expected: object = None
    actual: object = None

    @property
    def message(self):
        return 'equal' if self.passed else f'{self.field}: reference {self.expected!r}, word check {self.actual!r}'


def compare_step(check, reference_row, mapping):
    """Compare all mapped fields, refusing missing fields and partial replay."""
    if not check.admissible:
        return Comparison(False, f'letter {check.failing_letter_index}', 'admissible', check.failing_status)
    if not mapping:
        return Comparison(False, 'mapping', 'non-empty field mapping', mapping)
    for reference, field in mapping.items():
        if reference not in reference_row:
            return Comparison(False, reference, 'present reference field', '<missing>')
        actual, expected = getattr(check, field), reference_row[reference]
        if type(actual) is not type(expected) or actual != expected:
            return Comparison(False, reference, expected, actual)
    return Comparison(True)


def units_at(state):
    """Recompute eb4_replay's units, one frame part per block, in block order."""
    _research()
    import numpy as np
    import blocks
    import rim
    import seams

    st = copy_state(state)
    model = blocks.Blocks(seams.SeamRestorer(st, _menu(st)), _menu(st))
    dom = np.asarray(rim.Domains(st).dom)
    units = []
    for pieces in model.blocks:
        pieces = np.asarray(pieces)
        for d in np.unique(dom[pieces]).tolist():
            units.append(set(pieces[dom[pieces] == d].tolist()))
    return units


def _json(path):
    raw = Path(path).read_bytes()
    return json.loads(gzip.decompress(raw) if Path(path).suffix == '.gz' else raw)


def _extracted(directory, relative):
    path = (directory / relative).resolve()
    if not path.is_relative_to(directory) or path.is_relative_to(ROOT):
        raise ValueError('extracted input must stay beneath --extract and outside the repository')
    return path


def _table(path):
    census = _research()
    import orient
    import sim

    state = sim.State(census.restore.ctx, menu=census.menu())
    orient.set_pose_table(state, _json(path))
    return state


def _program(directory, path, menu):
    from differential.menu_order import inverse_index

    inverse = inverse_index(menu.record())

    def invert(word):
        return [[c, inverse[i]] for c, i in reversed(word)]

    word = []
    for index, segment in enumerate(_json(path)['segments']):
        kinds = [k for k in ('word', 'word_file', 'safe_word_file', 'core_file') if k in segment]
        if len(kinds) != 1:
            raise ValueError(f'program segment {index}: requires a literal word or core')
        kind = kinds[0]
        if kind == 'word':
            part = segment[kind]
        else:
            doc = _json(_extracted(directory, segment[kind]))
            part = doc['P'] + doc['Q'] + invert(doc['P']) if kind == 'core_file' else doc['word']
        word.extend(invert(part) if segment.get('inverse', False) else part)
    return _letters(word, menu)


def case_input(name, extract=None):
    """Known recorded inputs only; no search, solver, or draft-branch reads."""
    census = _research()
    if name == 'u1':
        start = census.state_at(7101, 98).st
        doc = _json(ROOT / LOW / 'runs/lc13-k40/steps/u1.json')
        reference = _json(ROOT / LOW / 'runs/lc13-k40/replay-b01.json')
        if doc['input_digest'] != start.digest():
            raise ValueError(f"u1 input_digest: expected {doc['input_digest']}, actual {start.digest()}")
        row = dict(reference['rows'][0], start_digest=reference['digest'], off_before=reference['off'],
                   end_digest=doc['output_digest'])
        return start, doc['word'], units_at(start)[1], row, STEP_MAPPING
    if extract is None:
        raise FileNotFoundError(f'{name} requires --extract with the recorded draft inputs')
    extract = Path(extract).resolve()
    if extract.is_relative_to(ROOT):
        raise ValueError('--extract must be outside the repository')
    base = LOW + 'runs/round22/'

    def path(relative):
        return _extracted(extract, base + relative)

    def unit(k):
        return set(_json(path(f'line-a/units/eb5-k40-k00-b{k:02d}.json'))['pieces'])

    if name == 'b20':
        state = _table(path('line-a/literal/run1/z-02-b16.json.gz'))
        word = _program(extract, path('line-a/literal/run1/03-b20-program.json'), census.menu())
        row = _json(path('line-a/literal/run1/03-b20-stepchk.json'))
        return state, word, unit(20), row, STEP_MAPPING
    if name == 'b06-14':
        # 336,206 is V's length; the entire step X W V W^-1 V^-1 is 672,417.
        state = _table(path('line-a/literal/run2b/z-04-b15.json.gz'))
        word = _program(extract, path('line-a/literal/run3/t1/b06-14-program.json'), census.menu())
        row = json.loads(path('line-a/literal/run3/t1/b06-14-apply.log').read_text().splitlines()[-1])
        return state, word, unit(6), row, {k: v for k, v in STEP_MAPPING.items() if k != 'letters'}
    if name == 'b14':
        from differential.j1_state import twists_from_letters

        state = copy_state(census.state_at(7101, 98).st)
        for index, twist in enumerate(twists_from_letters(census.menu(), _json(path('t1prime-k40/steps/u12.json'))['word'])):
            outcome = state.apply(twist, journal=False)
            if not outcome.applied:
                raise ValueError(f'b12 setup letter {index}: {outcome.status}')
        expected = _json(path('line-c/b14-after-b12/b12-replay.json'))['end_digest']
        if state.digest() != expected:
            raise ValueError(f'b12 setup digest: expected {expected}, actual {state.digest()}')
        word = _json(path('line-a/census-class3.json'))['units']['eb5-k40-k00-b14']['zero_bad']['word']
        return state, word, unit(14), {'collateral': 17, 'strict_prime': False}, {
            'collateral': 'lattice_pieces_lifted_off', 'strict_prime': 'step_flag'}
    if name == 'b03':
        state = _table(path('line-a/replay/tables/737.json.gz'))
        if state.digest() != '2e3b8f990229799fc88198b2fe4f2062ab4f9d73c78347ee67b252d412b0a086':
            raise ValueError('b03: extracted 737 table has a different start digest')
        word = _json(path('t1prime-k40/steps/u3.json'))['word']
        return state, word, unit(3), {'strict_prime': False}, {'strict_prime': 'step_flag'}
    if name == 'r22b34':
        raise FileNotFoundError('R22B-34 specifies wrong inverse order and dropped frame rotations as controls; '
                                'it does not identify saved negative words or their expected labelled actions. '
                                'No recorded negative fixture can be reconstructed from that specification alone.')
    raise ValueError(f'unknown case {name}')


def run_cases(cases, extract=None):
    rows = []
    for name in cases:
        started = perf_counter()
        try:
            state, word, unit, reference, mapping = case_input(name, extract)
            check = word_check(state, word, unit)
            comparison = compare_step(check, reference, mapping)
            row = dict(case=name, status='passed' if comparison.passed else 'failed',
                       word_check=asdict(check), reference_row=reference, mapping=mapping,
                       comparison=asdict(comparison))
        except FileNotFoundError as error:
            row = dict(case=name, status='unavailable', reason=str(error), comparison=None)
        except (KeyError, ValueError, AssertionError) as error:
            # A recorded input that does not rebuild (a start digest, a setup step) is a failure.
            row = dict(case=name, status='error', reason=f'{type(error).__name__}: {error}', comparison=None)
        row['seconds'] = perf_counter() - started
        rows.append(row)
    return rows, dict(passed=bool(rows) and all(r['status'] == 'passed' for r in rows),
                      compared=sum(r['comparison'] is not None for r in rows),
                      unavailable=[r['case'] for r in rows if r['status'] == 'unavailable'],
                      errors=[r['case'] for r in rows if r['status'] == 'error'])


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True)
    parser.add_argument('--extract')
    parser.add_argument('--cases', nargs='+', choices=('u1', 'b20', 'b06-14', 'b14', 'b03', 'r22b34'), default=['u1'])
    args = parser.parse_args(argv)
    # Validate every path before NumPy/J1/research imports or cache creation.
    out = Path(args.out).resolve()
    if out.is_relative_to(ROOT):
        parser.error('--out must be outside the repository')
    extract = None if args.extract is None else Path(args.extract).resolve()
    if extract is not None and extract.is_relative_to(ROOT):
        parser.error('--extract must be outside the repository')
    names = ('word-checks.jsonl', 'word-summary.json')
    for name in names:
        if (out / name).resolve().is_relative_to(ROOT):
            parser.error(f'--out must be outside the repository: {name} resolves inside it')
    rows, summary = run_cases(args.cases, extract)
    out.mkdir(parents=True, exist_ok=True)
    (out / names[0]).write_text(''.join(json.dumps(row) + '\n' for row in rows), encoding='utf-8')
    (out / names[1]).write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
    return int(not summary['passed'])


if __name__ == '__main__':
    sys.exit(main())
