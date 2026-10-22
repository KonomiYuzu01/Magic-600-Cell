"""Independent literal lattice replay for runner-run/3 (headless research).

Only puzzle definitions are shared: the exact simulator, model assets and I_a
menu. Retained letters on the lattice gather sticker labels and move exact K+
poses by occupied place. Every other letter uses State.apply. No net word
permutation advances either representation.
"""

from functools import lru_cache
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import sys
import time
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
CHUNK_BYTES = 1 << 20
TREE_MAX_DEPTH = 64


class Sha256Mismatch(ValueError):
    pass


class Ia16Error(ValueError):
    """A sanitized literal-file failure, independent of exception/path text."""
    def __init__(self, name):
        self.name = name
        super().__init__(name)


def ia16_chunks(path, *, sha=None):
    """Yield little-endian uint16 chunks, carrying a split code across reads.

    Each gzip read is bounded by CHUNK_BYTES, looked up on entry. Exhausting the
    iterator validates the trailer, byte count and every code, including suffixes
    that a range reader does not retain.
    """
    chunk_bytes = CHUNK_BYTES
    if type(chunk_bytes) is not int or chunk_bytes < 1:
        raise Ia16Error('word_file')
    try:
        with Path(path).open('rb') as raw:
            header = raw.read(8)
            if len(header) != 8 or header[:2] != b'\x1f\x8b' or header[4:8] != bytes(4):
                raise Ia16Error('word_file')
            raw.seek(0)
            with gzip.GzipFile(fileobj=raw, mode='rb') as stream:
                carry = b''
                while data := stream.read(chunk_bytes):
                    if sha is not None:
                        sha.update(data)
                    data = carry + data
                    end = len(data) - len(data) % 2
                    carry = data[end:]
                    codes = np.frombuffer(data[:end], dtype='<u2')
                    if codes.size and int(codes.max()) >= GRIPS * ITEMS:
                        raise Ia16Error('word_literal')
                    if codes.size:
                        yield codes
                if carry:
                    raise Ia16Error('word_file')
    except Ia16Error:
        raise
    except (OSError, EOFError, ValueError, zlib.error):
        raise Ia16Error('word_file') from None


def ia16_info(path):
    sha, letters = hashlib.sha256(), 0
    for codes in ia16_chunks(path, sha=sha):
        letters += len(codes)
    return {'letters': letters, 'sha256': sha.hexdigest()}


def read_ia16(path, sha256=None):
    """Compatibility reader for non-lattice steps: return [[grip, item], ...]."""
    word, sha = [], hashlib.sha256()
    for codes in ia16_chunks(path, sha=sha):
        word.extend([code // ITEMS, code % ITEMS] for code in codes.tolist())
    if sha256 is not None and sha.hexdigest() != sha256:
        raise Sha256Mismatch('word_sha256')
    return word


def ia16_ranges(path, ranges, *, info=None):
    """Read ranges in one separate pass, sharing storage for overlaps.

    A bytearray retains at most the prefix through the furthest requested end;
    all returned arrays are uint16 views of that one buffer. It grows only from
    bytes actually read, so an invalid large end cannot allocate a large array.
    """
    if (not isinstance(ranges, list) or any(
            not isinstance(r, (tuple, list)) or len(r) != 2
            or any(type(v) is not int or v < 0 for v in r) or r[1] < r[0]
            for r in ranges)):
        raise Ia16Error('word_file')
    end = max((r[1] for r in ranges), default=0)
    data, at, sha = bytearray(), 0, hashlib.sha256()
    for codes in ia16_chunks(path, sha=sha):
        if at < end:
            data.extend(codes[:end - at].tobytes())
        at += len(codes)
    if end > at:
        raise Ia16Error('word_file')
    if info is not None:
        info.update(letters=at, sha256=sha.hexdigest())
    codes = np.frombuffer(data, dtype='<u2')
    return [codes[lo:hi] for lo, hi in ranges]


def _add(fails, name):
    if name not in fails:
        fails.append(name)


def check_tree_ranges(path, root, menu, *, expected=None):
    """Validate ranges iteratively, then compare all backward references."""
    try:
        seen, references, at = {}, [], 0
        stack = [(root, (), 1)]
        while stack:
            part, index, depth = stack.pop()
            if (not isinstance(part, dict) or depth > TREE_MAX_DEPTH
                    or not isinstance(part.get('name'), str) or not part['name']
                    or type(part.get('letters')) is not int or part['letters'] < 0
                    or ('span' in part) == ('parts' in part)
                    or ('inverse_of' in part and 'same_as' in part)):
                return ['tree_malformed']
            lo, hi = at, at + part['letters']
            if 'span' in part:
                span = part['span']
                if (not isinstance(span, list) or len(span) != 2
                        or any(type(v) is not int or v < 0 for v in span)
                        or span != [lo, hi]):
                    return ['tree_malformed']
                at = hi
            else:
                children = part['parts']
                if (not isinstance(children, list) or not children
                        or any(not isinstance(c, dict) or type(c.get('letters')) is not int
                               or c['letters'] < 0 for c in children)
                        or sum(c['letters'] for c in children) != part['letters']):
                    return ['tree_malformed']
                stack.extend((children[i], index + (i,), depth + 1)
                             for i in reversed(range(len(children))))
            for key in ('inverse_of', 'same_as'):
                if key not in part:
                    continue
                ref = part[key]
                if not isinstance(ref, list) or any(type(i) is not int or i < 0 for i in ref):
                    return ['tree_malformed']
                other = seen.get(tuple(ref))
                if (other is None or other[1] > lo or other[1] - other[0] != hi - lo
                        or (key == 'same_as' and other[2] != part['name'])):
                    return ['tree_malformed']
                references.append((key, (lo, hi), other[:2]))
            seen[index] = (lo, hi, part['name'])
        expected = ia16_info(path) if expected is None else expected
        if at != root['letters'] or expected['letters'] != root['letters']:
            return ['tree_malformed']
        # Deduplicate ranges; nested references can cover the same word many times.
        ranges = list(dict.fromkeys(r for _, mine, theirs in references for r in (mine, theirs)))
        info = {}
        values = dict(zip(ranges, ia16_ranges(path, ranges, info=info)))
        fails = []
        if info['letters'] != expected['letters']:
            _add(fails, 'letters')
        if info['sha256'] != expected['sha256']:
            _add(fails, 'word_sha256')
        if fails:
            return fails
        lookup = {restore.to_tuple(m): i for i, (_, m) in enumerate(menu.items)}
        inverse = np.array([lookup[restore.to_tuple(restore.transpose(m))]
                            for _, m in menu.items], dtype=np.uint16)
        for key, mine, theirs in references:
            actual, expected = values[mine], values[theirs]
            # Bound temporary inverse arrays too, including for one enormous range.
            width = max(1, CHUNK_BYTES // 2)
            for start in range(0, len(actual), width):
                end = min(start + width, len(actual))
                if key == 'inverse_of':
                    source = expected[len(actual) - end:len(actual) - start][::-1]
                    comparison = ITEMS * (source // ITEMS) + inverse[source % ITEMS]
                else:
                    comparison = expected[start:end]
                if not np.array_equal(actual[start:end], comparison):
                    _add(fails, 'tree_expansion')
                    break
        return fails
    except Ia16Error as exc:
        return [exc.name]


class FastPathError(ValueError):
    pass


class ModelLabelsError(ValueError):
    pass


@lru_cache(maxsize=1)
def _fast_items():
    menu = restore.menu('I_a')
    return frozenset(i for i, (_, matrix) in enumerate(menu.items)
                     if restore.ctx.kplus.index_of_matrix(matrix) >= 0)


def fast_letter(code, on_lattice):
    """Dispatcher predicate; the model independently guards its own menu table."""
    return (on_lattice and 0 <= code < GRIPS * ITEMS and code % ITEMS in _fast_items())


class _Moves:
    """Lazy primitive moves and exact left-composition tables from the simulator."""
    def __init__(self, ctx):
        self.ctx = ctx
        self.src, self.dst, self.offsets = ctx.data.primitives()
        self.instructions, self.words = {}, {}

    def instruction(self, mid):
        cached = self.instructions.get(mid)
        if cached is None:
            data, kp = self.ctx.data, self.ctx.kplus
            j = abs(mid) - 1
            a, b = self.offsets[j:j + 2]
            src, dst = self.src[a:b], self.dst[a:b]
            ps, pd = data.move_src[j], data.move_dst[j]
            r = int(kp.gen_idx[j])
            if mid < 0:
                src, dst, ps, pd = dst, src, pd, ps
                r = int(kp.inv[r])
            k = np.arange(len(kp.perms), dtype=np.int64)
            left = kp.compose_vec(np.full(len(k), r), k).astype(np.int16)
            cached = self.instructions[mid] = src, dst, ps, pd, left
        return cached

    def word(self, grip, base):
        key = (grip, base)
        if key not in self.words:
            kp = self.ctx.kplus
            # Exact index of F_c * item * F_c^-1, the menu's element at c.
            f = int(kp.frame_idx[grip])
            g = kp.compose(f, kp.compose(base, int(kp.inv[f])))
            self.words[key] = tuple(self.instruction(mid) for mid in kp.a4_word(grip, g))
        return self.words[key]


@lru_cache(maxsize=1)
def _moves(ctx):
    return _Moves(ctx)


class LatticeModel:
    """Labels plus K+ poses held at occupied places, including invisible rotations."""
    @classmethod
    def from_state(cls, state, menu):
        if not state.is_lattice():
            raise FastPathError('off lattice')
        self = cls()
        self.ctx, self.menu = state.ctx, menu
        stickers = state.lattice_stickers()
        if not np.all(stickers['frame_agrees']):
            raise ModelLabelsError('model_labels')
        self.labels = stickers['labels'].copy()
        kp, reg = self.ctx.kplus, self.ctx.regions
        k = state._kpose()[state.pose_id]
        place = reg.piece_of[reg.orbit_of, kp.compose_vec(k, reg.transport)]
        if not np.array_equal(np.sort(place), np.arange(len(k))):
            raise ModelLabelsError('model_labels')
        self.place_pose = np.empty(len(k), dtype=np.int16)
        self.place_pose[place] = k
        # This table is built from the supplied menu, never from fast_letter.
        self._base = tuple(kp.index_of_matrix(matrix) for _, matrix in menu.items)
        self._moves = _moves(self.ctx)
        return self

    def play(self, code):
        if (not isinstance(code, (int, np.integer)) or isinstance(code, (bool, np.bool_))
                or not 0 <= code < GRIPS * ITEMS):
            raise FastPathError('invalid code')
        grip, item = divmod(int(code), ITEMS)
        if item >= len(self._base) or self._base[item] < 0:
            raise FastPathError('non-retained item')
        for src, dst, ps, pd, left in self._moves.word(grip, self._base[item]):
            self.labels[dst] = self.labels[src]
            self.place_pose[pd] = left[self.place_pose[ps]]

    def to_state(self):
        """Return the exact rebuilt state and whether its labels and frames agree."""
        data = self.ctx.data
        if (not np.array_equal(np.sort(self.labels), np.arange(len(data.slot_piece)))
                or self.place_pose.shape != (len(data.mask_offsets) - 1,)
                or np.any(self.place_pose < 0) or np.any(self.place_pose >= len(self.ctx.kplus.perms))):
            raise ModelLabelsError('model_labels')
        home = data.slot_piece[self.labels]
        pose_at_slot = self.place_pose[data.slot_piece]
        home_pose = np.empty(len(self.place_pose), dtype=np.int16)
        home_pose[home] = pose_at_slot
        if not np.array_equal(home_pose[home], pose_at_slot):
            raise ModelLabelsError('model_labels')
        state = sim.State(self.ctx, menu=self.menu)
        unique, inverse = np.unique(home_pose, return_inverse=True)
        remap = np.array([state._intern_k(int(k)) for k in unique], dtype=np.int32)
        state.pose_id = remap[inverse]
        state._compact()
        try:
            stickers = state.lattice_stickers()
            agrees = bool(np.all(stickers['frame_agrees'])
                          and np.array_equal(stickers['labels'], self.labels))
        except (ValueError, AssertionError, KeyError):
            agrees = False
        return state, agrees


def _inside(base, name):
    if (not isinstance(name, str) or not name or '\\' in name or '\x00' in name
            or PurePosixPath(name).is_absolute() or PureWindowsPath(name).drive
            or '..' in name.split('/')):
        raise ValueError('path')
    path = (base / name).resolve()
    if not path.is_relative_to(base):
        raise ValueError('path')
    return path


def _no_duplicates(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate key')
        result[key] = value
    return result


def _step_tree(path, row):
    """Recover the run root from the word's relative path; never import a player."""
    try:
        word = row.get('word')
        if not isinstance(word, str):
            return None, ['path']
        n = len(PurePosixPath(word).parts)
        base = Path(path).parents[n - 1].resolve() if n else None
        if base is None or _inside(base, word) != Path(path).resolve():
            return None, ['path']
    except (ValueError, IndexError, OSError, RuntimeError):
        return None, ['path']
    tree = row.get('tree')
    if not isinstance(tree, dict) or set(tree) != {'file', 'sha256', 'size'}:
        return None, ['tree_missing']
    try:
        tree_path = _inside(base, tree['file'])
    except (ValueError, OSError, RuntimeError):
        return None, ['path']
    try:
        raw = tree_path.read_bytes()
    except OSError:
        return None, ['tree_missing']
    try:
        if len(raw) < 8 or raw[:2] != b'\x1f\x8b' or raw[4:8] != bytes(4):
            raise ValueError('gzip header')
        data = gzip.decompress(raw)
    except (ValueError, OSError, EOFError, zlib.error):
        return None, ['tree_malformed']
    if hashlib.sha256(data).hexdigest() != tree.get('sha256'):
        return None, ['tree_sha256']
    try:
        return json.loads(data, object_pairs_hook=_no_duplicates), []
    except (ValueError, RecursionError):
        return None, ['tree_malformed']


def _replace_state(target, source):
    target.pose_id, target._poses, target._lookup = source.pose_id, source._poses, source._lookup
    target.journal, target._anchor_cache = [], {}


def replay_lattice_step(state, row, path, menu, *, stats=None):
    """Verify files before play, replay literally, and leave the exact state in place.

    Counts in stats describe successfully played letters. Switches count each
    conversion into or out of the model, including the final write-back. The
    timer is diagnostic only; it never limits replay.
    """
    started = time.perf_counter()
    counts = {'letters': 0, 'fast': 0, 'simulated': 0, 'switches': 0,
              'labels_identity': None, 'digest': None, 'seconds': 0.0}
    fails, model = [], None

    def flush():
        nonlocal model
        if model is not None:
            current, model = model, None
            rebuilt, agrees = current.to_state()
            _replace_state(state, rebuilt)
            if not agrees:
                _add(fails, 'model_labels')
            counts['switches'] += 1

    try:
        if row.get('encoding') != 'ia16':
            _add(fails, 'encoding')
        if type(row.get('letters')) is not int or row['letters'] < 0:
            _add(fails, 'letters')
        if (not isinstance(row.get('word_sha256'), str)
                or re.fullmatch('[0-9a-f]{64}', row['word_sha256']) is None):
            _add(fails, 'word_sha256')
        if fails:
            return fails
        info = ia16_info(path)
        if info['letters'] != row['letters']:
            _add(fails, 'letters')
        if info['sha256'] != row['word_sha256']:
            _add(fails, 'word_sha256')
        if fails:
            return fails
        root, tree_fails = _step_tree(path, row)
        if not tree_fails:
            tree_fails = check_tree_ranges(path, root, menu, expected=info)
        for name in tree_fails:
            _add(fails, name)

        on_lattice = state.is_lattice()
        replay_sha = hashlib.sha256()
        for codes in ia16_chunks(path, sha=replay_sha):
            for code in codes.tolist():
                if fast_letter(code, on_lattice):
                    if model is None:
                        model = LatticeModel.from_state(state, menu)
                        counts['switches'] += 1
                    model.play(code)
                    counts['fast'] += 1
                else:
                    flush()
                    cap, item = divmod(code, ITEMS)
                    try:
                        twist = sim.Twist(state.ctx, cap, restore.element(menu, cap, item),
                                          'menu', {'menu': menu.name})
                        outcome = state.apply(twist, journal=False)
                        admitted = outcome.applied and not outcome.uncertain
                    except Exception:
                        admitted = False
                    if not admitted:
                        _add(fails, 'admissible')
                        return fails
                    counts['simulated'] += 1
                    on_lattice = state.is_lattice()
                counts['letters'] += 1
        if counts['letters'] != row['letters']:
            _add(fails, 'letters')
        if replay_sha.hexdigest() != row['word_sha256']:
            _add(fails, 'word_sha256')
        flush()
        off, end_digest = state.off_lattice_count(), state.digest()
        after = row.get('after')
        after = after if isinstance(after, dict) else {}
        if type(after.get('off')) is not int or after['off'] != 0 or after['off'] != off:
            _add(fails, 'after_off')
        if after.get('digest') != end_digest:
            _add(fails, 'after_digest')
        identity = False
        if not off:
            stickers = state.lattice_stickers()
            frames = bool(np.all(stickers['frame_agrees']))
            if not frames:
                _add(fails, 'model_labels')
            identity = frames and np.array_equal(stickers['labels'], np.arange(len(stickers['labels'])))
        counts['labels_identity'], counts['digest'] = bool(identity), end_digest
        if 'labels_identity' in row and (type(row['labels_identity']) is not bool
                                          or row['labels_identity'] != identity):
            _add(fails, 'labels_identity')
    except Ia16Error as exc:
        _add(fails, exc.name)
    except FastPathError:
        _add(fails, 'fast_path')
    except ModelLabelsError:
        _add(fails, 'model_labels')
    except Exception:
        _add(fails, 'lattice_step')
    finally:
        try:
            flush()
        except ModelLabelsError:
            _add(fails, 'model_labels')
        except Exception:
            _add(fails, 'lattice_step')
        counts['seconds'] = time.perf_counter() - started
        if isinstance(stats, dict):
            stats.update(counts)
    return fails
