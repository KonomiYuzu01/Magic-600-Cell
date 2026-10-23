"""Plugin interface of the tutorial runner, format runner-run/3 (headless research).

The runner owns the state, the record and the fixed phase order. A phase plugin
reads the current state and proposes one step, or ends its phase; the runner
plays every proposed word twist by twist, checks the claimed end digest and
records the step with its tree of named parts. The lattice plugin plays its whole
phase in a child and returns a LatticeResult; the runner checks its words, trees
and digest chain and its end table against the last step. Nothing in this module
searches.
"""
import array
from dataclasses import dataclass, field
import gzip
import hashlib
import json
import os
from pathlib import Path
import sys

FORMAT = 'runner-run/3'
PHASE_ORDER = ('bulk', 'height', 'high', 'tail', 'lattice')
SUBPHASES = {'height': ('r22d75', 'r22d81', 'h4')}
RESULTS = ('lattice', 'handoff', 'not_applicable', 'stuck', 'budget', 'timeout', 'error')
STANDINGS = ('rule', 'certificate', 'fallback')
MODES = ('acceptance', 'development')
# A child clock argument cannot be null, and most children reject infinity
# (stage 13 README, section 1). Acceptance mode passes this finite sentinel.
CLOCK_SENTINEL = 1e9
TREE_MAX_DEPTH = 64
FORBIDDEN_KEYS = frozenset({'segments', 'slp', 'certificate', 'node'})
GRIPS, ITEMS = 600, 60


@dataclass
class Proposal:
    """One literal step. ``word`` is {'word', 'input_digest', 'output_digest'}."""
    phase: str
    subphase: str | None
    rule: str
    word: dict
    contract: str
    gate: str
    tree: dict | None = None
    fields: dict = field(default_factory=dict)
    standing: str = 'rule'
    attempts: list = field(default_factory=list)


@dataclass
class End:
    result: str
    reason: str | None = None
    details: dict = field(default_factory=dict)
    attempts: list = field(default_factory=list)

    def __post_init__(self):
        if self.result not in RESULTS:
            raise ValueError('unknown phase result')


@dataclass
class LatticeStep:
    """One played lattice step: a cascade orbit or a correction macro (packet P3). Its word and
    tree are files the plugin's child wrote; ``before`` and ``after`` are lattice snapshots."""
    rule: str
    word_file: Path
    word_sha256: str
    letters: int
    tree_file: Path
    tree_sha256: str
    tree_size: int
    before: dict
    after: dict
    fields: dict
    seconds: float
    contract: str = 'lattice_literal'
    gate: str = 'player'
    standing: str = 'rule'
    encoding: str = 'ia16'


@dataclass
class LatticeResult:
    """The lattice plugin's played phase: its steps in played order, the end table and its report."""
    steps: list
    end_table: Path
    report: dict


@dataclass
class Ctx:
    """What a plugin may use: the current state and its table, a fresh snapshot,
    the runner's child executor and its clocks. ``execute(argv, tag, seconds,
    cwd=None, stop_when=None)`` returns the runner's Child; ``seconds=None``
    means no clock. A plugin reads the state; only the runner changes it."""
    table: Path
    state: object
    snapshot: object
    mechanics: object
    work_dir: Path
    cache: Path
    execute: object
    budgets: dict
    mode: str = 'development'
    subphase: str | None = None

    def wall(self, name):
        """The runner's own wall clock for a call: None (no clock) in acceptance mode."""
        return None if self.mode == 'acceptance' else self.budgets.get(name)

    def child_clock(self, name):
        return child_clock(self.mode, self.budgets.get(name))


def child_clock(mode, value):
    """The value a child program receives for a clock argument."""
    if mode not in MODES:
        raise ValueError('unknown mode')
    if mode == 'acceptance' or value is None:
        return CLOCK_SENTINEL
    return value


def recorded_budgets(mode, budgets, clocks):
    """config.budgets: null for every clock that acceptance mode lifts."""
    return {name: (None if mode == 'acceptance' and name in clocks else value)
            for name, value in budgets.items()}


# Step trees. A part is {name, letters, parts: [...]} or {name, letters, span: [from, to]};
# it may carry inverse_of or same_as, the index path from the root to an earlier part.

def span(name, start, end):
    if not (isinstance(name, str) and name and 0 <= start <= end):
        raise ValueError('invalid span')
    return {'name': name, 'letters': end - start, 'span': [start, end]}


def group(name, parts):
    if not (isinstance(name, str) and name and parts):
        raise ValueError('invalid group')
    return {'name': name, 'letters': sum(p['letters'] for p in parts), 'parts': list(parts)}


def flat(name, pieces, start=0, inverse=None, same=None):
    """A root whose children are consecutive spans [(name, length), ...].

    ``inverse`` and ``same`` map a child's index to the index of an earlier child."""
    children, at = [], start
    for child_name, length in pieces:
        children.append(span(child_name, at, at + length))
        at += length
    for mapping, key in ((inverse or {}, 'inverse_of'), (same or {}, 'same_as')):
        for later, earlier in mapping.items():
            if not 0 <= earlier < later < len(children):
                raise ValueError('a reference must name an earlier part')
            children[later][key] = [earlier]
    return group(name, children)


def single(name, letters):
    return span(name, 0, letters)


def walk(root):
    """Depth-first parts with their index paths and depths."""
    stack = [(root, (), 1)]
    while stack:
        part, path, depth = stack.pop()
        yield part, path, depth
        for i in reversed(range(len(part.get('parts', ())))):
            stack.append((part['parts'][i], path + (i,), depth + 1))


def tree_size(root):
    return sum(1 for _ in walk(root))


def check_tree(root, letters):
    """The runner's own structural self-check before it writes a tree."""
    at, seen, size = 0, {}, 0
    for part, path, depth in walk(root):
        size += 1
        if depth > TREE_MAX_DEPTH or not isinstance(part.get('name'), str) or not part['name']:
            raise ValueError('tree name or depth')
        if (FORBIDDEN_KEYS.intersection(part) or ('span' in part) == ('parts' in part)
                or {'inverse_of', 'same_as'} <= set(part)):
            raise ValueError('tree part kind')
        start = at
        if 'span' in part:
            lo, hi = part['span']
            if lo != at or hi < lo or part['letters'] != hi - lo:
                raise ValueError('tree span')
            at = hi
        elif part['letters'] != sum(p['letters'] for p in part['parts']):
            raise ValueError('tree part sum')
        for key in ('inverse_of', 'same_as'):
            if key in part:
                # An earlier part that ends before this one starts, of the same length.
                other, first = seen.get(tuple(part[key]), (None, None))
                if other is None or first + other['letters'] > start or other['letters'] != part['letters'] or (
                        key == 'same_as' and other['name'] != part['name']):
                    raise ValueError('tree reference')
        seen[path] = part, start
    if at != letters or root['letters'] != letters:
        raise ValueError('tree length')
    return size


def write_tree(path, root, letters):
    size = check_tree(root, letters)
    data = (json.dumps(root, separators=(',', ':'), allow_nan=False) + '\n').encode('utf-8')
    atomic_bytes(Path(path), gzip.compress(data, mtime=0))
    return {'sha256': hashlib.sha256(data).hexdigest(), 'size': size}


def read_tree(path):
    """A written tree's root and the sha256 of its uncompressed bytes."""
    raw = Path(path).read_bytes()
    if raw[:2] != b'\x1f\x8b' or raw[4:8] != bytes(4):
        raise ValueError('a tree file is gzip with mtime 0')
    data = gzip.decompress(raw)
    return json.loads(data), hashlib.sha256(data).hexdigest()


# ia16 words: gzip (mtime 0) of little-endian uint16 codes 60 * grip + item.

def ia16_code(grip, item):
    if not (type(grip) is int and type(item) is int and 0 <= grip < GRIPS and 0 <= item < ITEMS):
        raise ValueError('invalid I_a letter')
    return ITEMS * grip + item


class Ia16Writer:
    """Streams codes to a gzip file; the sha256 is over the uncompressed bytes."""
    def __init__(self, path):
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.temporary = self.path.with_name(self.path.name + '.tmp')
        self.raw = self.temporary.open('wb')
        self.file = gzip.GzipFile(filename='', mode='wb', fileobj=self.raw, mtime=0)
        self.sha = hashlib.sha256()
        self.count = 0

    def write_codes(self, codes):
        chunk = array.array('H', codes)
        if any(c >= GRIPS * ITEMS for c in chunk):
            raise ValueError('invalid ia16 code')
        if sys.byteorder != 'little':
            chunk.byteswap()
        data = chunk.tobytes()
        self.file.write(data)
        self.sha.update(data)
        self.count += len(chunk)

    def write_letters(self, letters):
        self.write_codes([ia16_code(g, i) for g, i in letters])

    def close(self):
        self.file.close()
        self.raw.flush()
        os.fsync(self.raw.fileno())
        self.raw.close()
        os.replace(self.temporary, self.path)
        return {'sha256': self.sha.hexdigest(), 'letters': self.count}


def ia16_digest(path):
    """What Ia16Writer.close returned for a word file, recomputed from the file: gzip with
    mtime 0 of whole uint16 codes, every code below 36,000."""
    with Path(path).open('rb') as f:
        header = f.read(8)
    if header[:2] != b'\x1f\x8b' or header[4:8] != bytes(4):
        raise ValueError('an ia16 word is gzip with mtime 0')
    sha, size, carry = hashlib.sha256(), 0, b''
    with gzip.open(path, 'rb') as f:
        while data := f.read(1 << 20):
            sha.update(data)
            size += len(data)
            data, carry = carry + data, b''
            if len(data) % 2:
                data, carry = data[:-1], data[-1:]
            chunk = array.array('H', data)
            if sys.byteorder != 'little':
                chunk.byteswap()
            if chunk and max(chunk) >= GRIPS * ITEMS:
                raise ValueError('invalid ia16 code')
    if carry:
        raise ValueError('an ia16 word holds whole uint16 codes')
    return {'sha256': sha.hexdigest(), 'letters': size // 2}


def atomic_bytes(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + '.tmp')
    with temporary.open('wb') as f:
        f.write(data)
        f.flush()
        os.fsync(f.fileno())
    os.replace(temporary, path)
