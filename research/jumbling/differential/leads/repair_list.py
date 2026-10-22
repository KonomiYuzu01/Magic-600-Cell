"""The research repair-list job at one caller-named cap (ENG-0J, EH-07).

This is a J1/reference cost lead, not a candidate engine or a cap chooser.
Only a completed result has rows; outcomes on an interrupted job are coverage,
never evidence that the list is empty.
"""

import sys

sys.dont_write_bytecode = True

from dataclasses import dataclass
from functools import lru_cache
import json
from pathlib import Path
import platform
import os
import subprocess
from time import perf_counter
from typing import Callable, Literal

# Import first: the restorer's optional centroid cache must be outside the repo.
from . import census
from differential.menu_order import I_A_IDENTITY, I_A_ORDER_DIGEST, check_menu
import restore

RepairRow = tuple[int, int, int, int, int]
PairOutcome = tuple[int, int, int | None, str | tuple[int, int]]
TIE_RULE = 'lowest value, then smaller pinned index'


@dataclass(frozen=True)
class RepairParameters:
    cap: int
    menu_identity: str
    order_digest: str
    tie_rule: str
    contract_revision: str


@dataclass(frozen=True)
class RepairCoverage:
    pairs_tried: int
    items_done: tuple[int, ...]
    meeting_caps: tuple[int, ...]


@dataclass(frozen=True)
class RepairTimings:
    per_x: dict[int, float]
    total: float


@dataclass(frozen=True)
class RepairListResult:
    status: Literal['completed', 'cancelled', 'cut_off', 'error']
    input_digest: str
    parameters: RepairParameters
    coverage: RepairCoverage
    rows: tuple[RepairRow, ...] | None
    outcomes: tuple[PairOutcome, ...]
    timings: RepairTimings
    message: str | None = None


@lru_cache(maxsize=1)
def contract_revision():
    fixture = census.JUMBLING / 'fixtures' / 'wj-I_a.json'
    return json.loads(fixture.read_text(encoding='utf-8'))['journal']['contract_revision']


def _caches(S):
    # These dictionaries are invalidated in place by apply/undo. Their values
    # (piece-id arrays, scores, balances) belong to this exact state. Domains'
    # pole/coset memoisation may grow while the same state is reused.
    return S._cls.copy(), S._sv.copy(), S._dom, S._bal.copy()


def _restore_caches(S, saved):
    S._cls, S._sv, S._dom, S._bal = saved[0].copy(), saved[1].copy(), saved[2], saved[3].copy()


def repair_list(S, c, *, max_pairs=None, cancel: Callable[[], bool] | None = None) -> RepairListResult:
    """List every lowering repair at c, in pinned x order then cap d order.

    The neighbours are the initially admissible, misaligned caps meeting c.
    Each x is applied once. After each y is undone, reuse the caches at x,
    including scores already obtained for other neighbours. No z is applied:
    M2 makes the sum of the three own-cut changes the exact delta N.
    """
    if max_pairs is not None and (type(max_pairs) is not int or max_pairs < 0):
        raise ValueError('max_pairs must be a non-negative integer or None')
    started = perf_counter()
    digest, moves = S.st.digest(), tuple(S.moves)
    check_menu(S.M.record(), identity=I_A_IDENTITY, order=I_A_ORDER_DIGEST)
    parameters = RepairParameters(c, I_A_IDENTITY, I_A_ORDER_DIGEST, TIE_RULE, contract_revision())
    outcomes, rows, done, per_x = [], [], [], {}
    neighbours = ()
    status, message = 'completed', None

    def finish():
        if tuple(S.moves) != moves or S.st.digest() != digest:
            raise AssertionError('repair_list did not restore the input state and moves')
        return RepairListResult(status, digest, parameters,
                                RepairCoverage(len(outcomes), tuple(done), neighbours),
                                tuple(rows) if status == 'completed' else None,
                                tuple(outcomes), RepairTimings(per_x, perf_counter() - started), message)

    if type(c) is not int or not 0 <= c < 600:
        status, message = 'error', 'cap must be an integer in 0..599'
        return finish()
    initial = S.scores(c)
    if initial is None or initial[0] == 0:
        status = 'error'
        message = f'cap {c} is blocked or uncertain' if initial is None else f'cap {c} is closed (N_c = 0), not misaligned'
        return finish()
    now_c, after_c = initial
    misaligned = set(S.survey())
    neighbours = tuple(sorted(({int(d) for d in restore.NEAR[c]} & misaligned) - {c}))
    items = [x for x in range(S.n_items) if S.g(c, x) != restore.I4]
    base = _caches(S)
    for x in items:
        if not neighbours:
            done.append(x)
            per_x[x] = 0.0
            continue
        # Check before opening as well, so a zero budget does no trial twists.
        if cancel is not None and cancel():
            status = 'cancelled'
            break
        if max_pairs is not None and len(outcomes) >= max_pairs:
            status = 'cut_off'
            break
        x_started = perf_counter()
        S.apply(c, x)
        try:
            for index, d in enumerate(neighbours):
                if index:
                    if cancel is not None and cancel():
                        status = 'cancelled'
                        break
                    if max_pairs is not None and len(outcomes) >= max_pairs:
                        status = 'cut_off'
                        break
                r = S.scores(d)
                delta, info = None, 'd_blocked_or_closed'
                if r is not None and r[0] > 0:
                    n_d, a_d = r
                    y = min(a_d, key=lambda k: (a_d[k], k))
                    info = 'd_no_drop'
                    if a_d[y] < n_d:
                        opened = _caches(S)
                        S.apply(d, y)
                        try:
                            rc = S.scores(c)
                            info = 'c_blocked_or_closed'
                            if rc is not None and rc[1]:
                                n_c2, a_c2 = rc
                                z = min(a_c2, key=lambda k: (a_c2[k], k))
                                delta = (after_c[x] - now_c) + (a_d[y] - n_d) + (a_c2[z] - n_c2)
                                info = (y, z)
                        finally:
                            S.undo()
                            _restore_caches(S, opened)
                outcomes.append((x, d, delta, info))
                if delta is not None and delta < 0:
                    rows.append((x, d, info[0], info[1], delta))
        finally:
            S.undo()
            _restore_caches(S, base)
            per_x[x] = perf_counter() - x_started
        if status != 'completed':
            break
        done.append(x)
    return finish()


def completed_rows(result: RepairListResult, *, step=None):
    """The sole list-reading boundary: interrupted/error results are not lists."""
    if result.status != 'completed' or result.rows is None:
        raise ValueError(f'step {step}: repair list is {result.status}, not completed')
    return result.rows


def _compare_rows(got, expected, *, step, kind):
    for index in range(max(len(got), len(expected))):
        a = got[index] if index < len(got) else '<missing>'
        b = expected[index] if index < len(expected) else '<missing>'
        if a != b:
            raise AssertionError(f'step {step}: first differing {kind} row {index + 1}: got {a!r}; recorded {b!r}')


def compare_recorded(result: RepairListResult, recorded, *, step):
    """Compare both listed rows and every outcome with density.py's rows."""
    rows = completed_rows(result, step=step)
    expected = tuple((r['x'], r['d'], r['delta'], tuple(r['info']) if isinstance(r['info'], list) else r['info'])
                     for r in recorded)
    listed = tuple((x, d, info[0], info[1], delta) for x, d, delta, info in expected
                   if delta is not None and delta < 0)
    _compare_rows(rows, listed, step=step, kind='listed')
    _compare_rows(result.outcomes, expected, step=step, kind='outcome')


def output_directory(path):
    """Validate before any output write, resolving existing symlink ancestors."""
    directory = Path(path).resolve()
    if directory.is_relative_to(census.ROOT):
        raise ValueError('--out must be outside the repository')
    return directory


def run_metadata():
    """Local, read-only provenance for a research timing."""
    import numpy as np
    try:
        head = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=census.ROOT,
                              capture_output=True, text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        head = None  # no Git checkout: the commit is unknown, never guessed
    return {'evidence_kind': 'source and fixture replay; cloud timings of research code',
            'cpu_count': os.cpu_count(), 'python': platform.python_version(),
            'numpy': np.__version__, 'commit': head}
