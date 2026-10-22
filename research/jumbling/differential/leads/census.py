"""Rebuild the states of the scramble-census records, step by step.

The census (`research/jumbling/restoration/11-scramble-census`) recorded each run as a scramble
(menu I_a, depth k, seed) and the moves of the research restorer, grouped in steps. `walk`
rebuilds a record as the census replay check does (`scripts/check_records.py`): the scramble from
(k, seed), checked against the record's history, then each step's twists, with N and the off
count checked before and after every step against the record. Every twist must be admissible.

The cost-lead scripts in this package measure at the yielded states. Evidence kind: source and
fixture replay with the J1 reference engine and the research restorer (`theory/restore.py`,
`theory/seams.py`); a timing measured here is a cloud timing of research code, never
performance evidence for a product engine.
"""

import sys

sys.dont_write_bytecode = True

import atexit
from dataclasses import dataclass
import json
from pathlib import Path
import shutil
import tempfile

ROOT = Path(__file__).resolve().parents[4]
JUMBLING = ROOT / 'research' / 'jumbling'
CENSUS = JUMBLING / 'restoration' / '11-scramble-census'
for _path in (JUMBLING / 'fixtures', JUMBLING, JUMBLING / 'theory'):
    if str(_path) not in sys.path:
        sys.path.insert(0, str(_path))

import restore  # noqa: E402
import seams  # noqa: E402

from differential.menu_order import I_A_ORDER_DIGEST, order_digest  # noqa: E402

# The latest record of each seed: round 2 continues round 1's runs of seeds 7101 to 7104, and
# round 3 holds three further runs. Each record holds every move from its scramble on.
RECORDS = {
    7001: 'runs/round1/ia-k10-s7001.json',
    7106: 'runs/round1/ia-k20-s7106.json',
    7107: 'runs/round1/ia-k40-s7107.json',
    7101: 'runs/round2/ia-k40-s7101.json',
    7102: 'runs/round2/ia-k100-s7102.json',
    7103: 'runs/round2/ia-k1000-s7103.json',
    7104: 'runs/round2/ia-k60-s7104.json',
    7105: 'runs/round3/ia-k200-s7105.json',
    7108: 'runs/round3/ia-k100-s7108.json',
    7109: 'runs/round3/ia-k400-s7109.json',
}

_MENU = None
_CACHE_DIR = None


class CensusError(AssertionError):
    """The rebuilt state differs from the record."""


def menu():
    """The research I_a menu, `wj.make_menu` through `restore.menu`, with the pinned item order."""
    global _MENU
    if _MENU is None:
        loaded = restore.menu('I_a')
        if order_digest(loaded.record()) != I_A_ORDER_DIGEST:
            raise CensusError('the research I_a menu does not have the pinned item order')
        _MENU = loaded
    return _MENU


def isolate_cache(path=None):
    """Point the research centroid cache (`seams.CACHE`, under `work/` by default) outside the
    repository: into `path`, or into a temporary directory removed at exit. Returns the file."""
    global _CACHE_DIR
    if path is None:
        if _CACHE_DIR is None:
            _CACHE_DIR = tempfile.mkdtemp(prefix='jumbling-census-')
            atexit.register(shutil.rmtree, _CACHE_DIR, True)
        path = _CACHE_DIR
    seams.CACHE = Path(path) / 'home_centroids.npy'
    return seams.CACHE


isolate_cache()


def load(seed_or_path):
    """(path, record) for a seed of RECORDS or a record path."""
    if isinstance(seed_or_path, int):
        path = CENSUS / RECORDS[seed_or_path]
    else:
        path = Path(seed_or_path)
    return path, json.loads(path.read_text(encoding='utf-8'))


def end_digest(path, record):
    """J1's end digest of a whole census record, as the census replay check printed it
    (`runs/check.out`, `runs/round3/check.out`), or None for a record outside the census runs."""
    try:
        rel = Path(path).resolve().relative_to(CENSUS / 'runs')
    except ValueError:
        return None
    out = CENSUS / 'runs' / ('round3' if rel.parts[0] == 'round3' else '') / 'check.out'
    for line in out.read_text(encoding='utf-8').splitlines():
        row = json.loads(line)
        if (row['record'], row['steps'], row['twists']) == (rel.name, len(record['steps']),
                                                            len(record['moves'])):
            return row['digest']
    raise CensusError(f'no end digest for {rel} in {out.relative_to(CENSUS)}')


@dataclass
class Step:
    """Step n (1-based) of a record, with the restorer S at the state before the step."""
    n: int
    rule: str
    word: list
    record: dict
    S: object


def scramble(record):
    """The research restorer at the record's scramble state, its history checked."""
    if record['menu'] != 'I_a':
        raise CensusError(f"record menu {record['menu']!r} is not I_a")
    st, hist = restore.scramble(menu(), record['k'], record['seed'])
    if [list(h) for h in hist] != [list(h) for h in record['history']]:
        raise CensusError('the rebuilt scramble differs from the record history')
    return seams.SeamRestorer(st, menu())


def walk(seed_or_path, last=None, end=False):
    """Yield a Step for steps 1 to `last` (default: every step) of a census record.

    Each Step's S is at the state before the step. The consumer may apply and undo twists on S
    but must hand it back as it found it; the walk then applies the step's word and checks N and
    the off count against the record. After the last step of a whole census record it also
    checks J1's end digest against the census replay check (a record outside the census runs
    has none to check), and with `end` it then yields one more Step at the end state:
    n = len(steps) + 1, rule None, an empty word and no record step."""
    path, record = load(seed_or_path)
    S = scramble(record)
    total = len(record['steps'])
    last = total if last is None else last
    if not 0 <= last <= total:
        raise ValueError(f'last step {last} is outside 0..{total}')
    pos = 0
    for j, step in enumerate(record['steps'][:last]):
        word = [tuple(m) for m in record['moves'][pos:pos + step['twists']]]
        pos += step['twists']
        N = S.misalignment()
        if N != step['N_before']:
            raise CensusError(f"step {j + 1}: N before is {N}, the record has {step['N_before']}")
        moves, digest = len(S.moves), S.st.digest()
        yield Step(j + 1, step['rule'], word, step, S)
        if len(S.moves) != moves or S.st.digest() != digest:
            raise CensusError(f'step {j + 1}: the consumer did not restore the state')
        for c, i in word:
            S.apply(c, i)
        got = (S.misalignment(), int(S.st.off_lattice_count()))
        if got != (step['N_after'], step['off_after']):
            raise CensusError(f"step {j + 1}: (N, off) after is {got}, the record has "
                              f"{(step['N_after'], step['off_after'])}")
    if last == total:
        want = end_digest(path, record)
        if want is not None and S.st.digest() != want:
            raise CensusError(f'{path.name}: the end digest differs from the census replay check')
        if end:
            yield Step(total + 1, None, [], None, S)


def state_at(seed_or_path, n):
    """The restorer at the state before step n of a record; n = len(steps) + 1 is its end."""
    path, record = load(seed_or_path)
    total = len(record['steps'])
    if not 1 <= n <= total + 1:
        raise ValueError(f'step {n} is outside 1..{total + 1}')
    for step in walk(path, last=min(n, total), end=n == total + 1):
        if step.n == n:
            return step.S
    raise CensusError(f'step {n} was not reached')
