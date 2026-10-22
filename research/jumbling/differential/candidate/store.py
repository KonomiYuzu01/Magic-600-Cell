"""Experimental jumbling session store; not the frozen product format.

Validation checks the representation, not admissibility, reachability, or the
truth of engine-supplied digests/certificates. Use ``canonical_digest`` with the
engine's K+ matrix lookup to check a digest, and ``expand_parts`` with the pinned
menu's inverse table to check inverse references. No reference-engine imports
are used here. Callers serialize writers/recovery for a given session path.
"""

import base64
import binascii
from collections.abc import Mapping
import copy
import errno
import hashlib
import json
import math
import os
from pathlib import Path
import struct
import tempfile


SESSION_FORMAT = "magic600-session-proto/1"
JUMBLING_FORMAT = "magic600-jumbling-store/1"
IDS_ENCODING = "u32le-base64"
_U32 = struct.Struct("<I")
_I32 = struct.Struct("<i")
_IDENTITY = [[[int(i == j), 0, 1] for j in range(4)] for i in range(4)]


class StoreFormatError(ValueError):
    """Invalid stored data; the message starts with its field path."""


class SimulatedCrash(RuntimeError):
    """Test-only interrupted write, deliberately leaving crash artifacts."""


def _fail(path, message):
    raise StoreFormatError(f"{path}: {message}")


def _mapping(value, path):
    if not isinstance(value, Mapping):
        _fail(path, "expected an object")
    return value


def _list(value, path):
    if not isinstance(value, list):
        _fail(path, "expected a list")
    return value


def _field(obj, key, path):
    if key not in obj:
        _fail(f"{path}.{key}", "missing field")
    return obj[key]


def _integer(value, path, minimum=0, maximum=None):
    if type(value) is not int or value < minimum or (
        maximum is not None and value > maximum
    ):
        _fail(path, "integer out of range (booleans are not integers)")
    return value


def _string(value, path):
    if not isinstance(value, str):
        _fail(path, "expected a string")
    return value


def _digest(value, path):
    # Lowercase only, as hashlib and the harness write them, so that digest
    # fields compare as strings.
    if (not isinstance(value, str) or len(value) != 64
            or any(c not in "0123456789abcdef" for c in value)):
        _fail(path, "expected a 64-digit lowercase SHA-256 hex digest")
    return value


def check_q5(x, _path="$"):
    """Check a reduced [a, b, d] for (a + b sqrt(5))/d, without bounds."""
    if not isinstance(x, list) or len(x) != 3:
        _fail(_path, "expected an exact triple [a, b, d]")
    for i, component in enumerate(x):
        if type(component) is not int:
            _fail(f"{_path}[{i}]", "Q5 components must be integers")
    if x[2] <= 0:
        _fail(f"{_path}[2]", "denominator must be positive")
    if math.gcd(*x) != 1:
        _fail(_path, "Q5 triple is not reduced")


def check_matrix(m, _path="$"):
    if not isinstance(m, list) or len(m) != 4:
        _fail(_path, "expected a 4 x 4 exact matrix")
    for i, row in enumerate(m):
        if not isinstance(row, list) or len(row) != 4:
            _fail(f"{_path}[{i}]", "expected four entries")
        for j, entry in enumerate(row):
            check_q5(entry, f"{_path}[{i}][{j}]")


def _int_text(value):
    # CPython 3.11+ limits decimal conversions by default. Do not change that
    # process-global setting: use bounded chunks for arbitrarily large entries.
    try:
        return str(value)
    except ValueError:
        negative = value < 0
        value = abs(value)
        chunks = []
        while value:
            value, remainder = divmod(value, 1_000_000_000)
            chunks.append(remainder)
        result = str(chunks.pop()) + "".join(f"{x:09d}" for x in reversed(chunks))
        return "-" + result if negative else result


def _parse_int(text):
    negative = text.startswith("-")
    digits = text[1:] if negative else text
    value = 0
    for at in range(0, len(digits), 9):
        chunk = digits[at:at + 9]
        value = value * 10 ** len(chunk) + int(chunk)
    return -value if negative else value


def _json_text(value, *, compact=True):
    separators = (",", ":") if compact else (", ", ": ")
    try:
        return json.dumps(value, sort_keys=True, separators=separators,
                          ensure_ascii=False, allow_nan=False)
    except ValueError:
        # Same JSON bytes, including Python's default matrix-key spacing, when
        # a decimal integer exceeds the interpreter's conversion guard.
        comma, colon = separators

        def render(item):
            if type(item) is int:
                return _int_text(item)
            if isinstance(item, list):
                return "[" + comma.join(render(v) for v in item) + "]"
            if isinstance(item, dict):
                return "{" + comma.join(
                    json.dumps(k, ensure_ascii=False) + colon + render(item[k])
                    for k in sorted(item)
                ) + "}"
            return json.dumps(item, ensure_ascii=False, allow_nan=False)

        return render(value)


def _check_json(value, path="$", active=None):
    """Opaque retained values and extra metadata must still be lossless JSON."""
    if value is None or type(value) in (bool, int):
        return
    if isinstance(value, str):
        try:
            value.encode("utf-8")
        except UnicodeError:
            _fail(path, "string is not valid UTF-8")
        return
    if type(value) is float:
        if not math.isfinite(value):
            _fail(path, "non-finite JSON number")
        return
    if not isinstance(value, (dict, list)):
        _fail(path, "expected a JSON value")
    if active is None:
        active = set()
    if id(value) in active:
        _fail(path, "cyclic JSON value")
    active.add(id(value))
    try:
        if isinstance(value, dict):
            for key, item in value.items():
                _string(key, path)
                _check_json(key, path, active)
                _check_json(item, f"{path}.{key}", active)
        else:
            for i, item in enumerate(value):
                _check_json(item, f"{path}[{i}]", active)
    finally:
        active.remove(id(value))


def _check_pose_table(table, path):
    _list(table, path)
    if not table or table[0] != {"k": 0}:
        _fail(f"{path}[0]", 'pose zero must be {"k": 0}')
    seen = set()
    for i, pose in enumerate(table):
        at = f"{path}[{i}]"
        _mapping(pose, at)
        if set(pose) == {"k"}:
            _integer(pose["k"], at + ".k", maximum=7199)
        elif set(pose) == {"m"}:
            check_matrix(pose["m"], at + ".m")
        else:
            _fail(at, "pose must have exactly one of k or m")
        key = _json_text(pose)
        if key in seen:
            _fail(at, "duplicate pose")
        seen.add(key)


def encode_ids(ids):
    """Pack pose ids or menu-letter codes as unsigned little-endian words."""
    raw = bytearray()
    count = 0
    for i, pid in enumerate(ids):
        _integer(pid, f"$[{i}]", maximum=2 ** 32 - 1)
        raw.extend(_U32.pack(pid))
        count += 1
    return {"encoding": IDS_ENCODING, "count": count,
            "sha256": hashlib.sha256(raw).hexdigest(),
            "data": base64.b64encode(raw).decode("ascii")}


def decode_ids(doc, n_poses, _path="$"):
    _mapping(doc, _path)
    _integer(n_poses, "$.n_poses")
    if _field(doc, "encoding", _path) != IDS_ENCODING:
        _fail(_path + ".encoding", "unknown id encoding")
    count = _integer(_field(doc, "count", _path), _path + ".count")
    digest = _digest(_field(doc, "sha256", _path), _path + ".sha256")
    data = _string(_field(doc, "data", _path), _path + ".data")
    try:
        raw = base64.b64decode(data, validate=True)
    except (ValueError, binascii.Error):
        _fail(_path + ".data", "invalid base64")
    if len(raw) != count * 4:
        _fail(_path + ".count", "count differs from byte length")
    if hashlib.sha256(raw).hexdigest() != digest.lower():
        _fail(_path + ".sha256", "id checksum mismatch")
    result = []
    for i, (pid,) in enumerate(struct.iter_unpack("<I", raw)):
        if pid >= n_poses:
            _fail(f"{_path}.data[{i}]", "id is outside the table")
        result.append(pid)
    return result


def canonical_digest(pose_table, ids, kplus_matrix):
    """J1-compatible exact-pose digest; piece numbers use signed int32 LE."""
    _check_pose_table(pose_table, "$.pose_table")
    groups = {}
    for piece, pid in enumerate(ids):
        _integer(pid, f"$.ids[{piece}]", maximum=len(pose_table) - 1)
        if pid:
            if piece >= 2 ** 31:
                _fail(f"$.ids[{piece}]", "piece index exceeds signed int32")
            groups.setdefault(pid, bytearray()).extend(_I32.pack(piece))
    # J1 interns poses, so no used pose repeats another's matrix or is the
    # identity; a table that breaks this would hash one pose as several items.
    seen = {_json_text(_IDENTITY, compact=False)}
    items = []
    for pid, members in groups.items():
        pose = pose_table[pid]
        matrix = kplus_matrix(pose["k"]) if "k" in pose else pose["m"]
        check_matrix(matrix, f"$.pose_table[{pid}]")
        key = _json_text(matrix, compact=False)
        if key in seen:
            _fail(f"$.pose_table[{pid}]", "used pose repeats another pose or the identity")
        seen.add(key)
        items.append((key, bytes(members)))
    digest = hashlib.sha256()
    for key, members in sorted(items):
        digest.update(key.encode("utf-8"))
        digest.update(members)
    return digest.hexdigest()


def check_record(rec, _path="$"):
    _mapping(rec, _path)
    _integer(_field(rec, "grip", _path), _path + ".grip", maximum=599)
    check_matrix(_field(rec, "matrix", _path), _path + ".matrix")
    _integer(_field(rec, "moved", _path), _path + ".moved")


def _check_parts(root, letters=None, inverse_index=None, path="$.parts"):
    """Check ranges/counts without allocating the word implied by the counts."""
    seen = {}

    def walk(part, indices, start, depth, at):
        if depth > 64:
            _fail(at, "part depth exceeds 64")
        _mapping(part, at)
        name = _string(_field(part, "name", at), at + ".name")
        if not name:
            _fail(at + ".name", "part name must not be empty")
        count = _integer(_field(part, "letters", at), at + ".letters")
        if ("parts" in part) == ("span" in part):
            _fail(at, "part must have exactly one of parts or span")
        if any(key in part for key in ("slp", "node", "segments", "certificate")):
            _fail(at, "reserved part key")
        if "inverse_of" in part and "same_as" in part:
            _fail(at, "part cannot have both inverse_of and same_as")
        reference = None
        kind = "inverse_of" if "inverse_of" in part else "same_as"
        if kind in part:
            ref_path = _list(part[kind], at + "." + kind)
            for i, index in enumerate(ref_path):
                _integer(index, f"{at}.{kind}[{i}]")
            reference = seen.get(tuple(ref_path))
            if reference is None:
                _fail(at + "." + kind, "reference must name an earlier completed part")
            other_name, lo, hi = reference
            if hi > start or hi - lo != count:
                _fail(at + "." + kind, "reference range or length differs")
            if kind == "same_as" and name != other_name:
                _fail(at + ".same_as", "reference name differs")
        if "span" in part:
            span = _list(part["span"], at + ".span")
            if len(span) != 2:
                _fail(at + ".span", "expected [from, to]")
            lo = _integer(span[0], at + ".span[0]")
            hi = _integer(span[1], at + ".span[1]")
            if lo != start or hi - lo != count:
                _fail(at + ".span", "spans must cover the word in depth-first order")
            end = hi
        else:
            end = start
            for i, child in enumerate(_list(part["parts"], at + ".parts")):
                end = walk(child, indices + (i,), end, depth + 1, f"{at}.parts[{i}]")
            if end - start != count:
                _fail(at + ".letters", "count differs from child counts")
        if letters is not None:
            if end > len(letters):
                _fail(at + ".letters", "part exceeds stored word")
            if reference is not None:
                _, lo, hi = reference
                for offset in range(count):
                    if kind == "same_as":
                        expected = letters[lo + offset]
                    elif inverse_index is not None:
                        grip, item = divmod(letters[hi - 1 - offset], len(inverse_index))
                        expected = grip * len(inverse_index) + inverse_index[item]
                    else:
                        # The document pins a menu identity, not its matrix table.
                        # Only expand_parts has the inverse table needed here.
                        break
                    if letters[start + offset] != expected:
                        _fail(at + "." + kind, "referenced letters differ")
        seen[indices] = (name, start, end)
        return end

    length = walk(root, (), 0, 1, path)
    if letters is not None and length != len(letters):
        _fail(path + ".letters", "root count differs from stored word")
    return length


def parts_length(root):
    """Exact expanded length from counts and spans, with no word allocation."""
    return _check_parts(root)


def expand_parts(root, letters, inverse_index):
    _list(letters, "$.letters")
    _list(inverse_index, "$.inverse_index")
    if not inverse_index:
        _fail("$.inverse_index", "empty menu")
    size = len(inverse_index)
    for i, inverse in enumerate(inverse_index):
        _integer(inverse, f"$.inverse_index[{i}]", maximum=size - 1)
    for i, inverse in enumerate(inverse_index):
        if inverse_index[inverse] != i:
            _fail(f"$.inverse_index[{i}]", "inverse table must be an involution")
    for i, letter in enumerate(letters):
        _integer(letter, f"$.letters[{i}]", maximum=600 * size - 1)
    _check_parts(root, letters, inverse_index)
    return list(letters)


def _identities(value, path):
    _mapping(value, path)
    model = _mapping(_field(value, "model", path), path + ".model")
    for name, digest in model.items():
        _string(name, path + ".model")
        _digest(digest, f"{path}.model.{name}")
    for key in ("menu", "menu_order"):
        _digest(_field(value, key, path), path + "." + key)
    _string(_field(value, "contract_revision", path), path + ".contract_revision")


def _retained_word(value, path):
    # J1's witness words use signed, 1-based primitive generator ids.
    for i, generator in enumerate(_list(value, path)):
        _integer(generator, f"{path}[{i}]", minimum=-1200, maximum=1200)
        if generator == 0:
            _fail(f"{path}[{i}]", "retained generator ids are signed and 1-based")


def _state(value, path, *, revision=False):
    _mapping(value, path)
    if revision:
        _integer(_field(value, "revision", path), path + ".revision")
    table = _field(value, "pose_table", path)
    _check_pose_table(table, path + ".pose_table")
    decode_ids(_field(value, "pose_ids", path), len(table), path + ".pose_ids")
    _digest(_field(value, "digest", path), path + ".digest")


def _records(value, path):
    for i, record in enumerate(_list(value, path)):
        check_record(record, f"{path}[{i}]")


def _certificate(value, path):
    _mapping(value, path)
    for key in ("pose_digest", "labels_digest", "certificate_program_sha256"):
        _digest(_field(value, key, path), path + "." + key)
    for key in ("c5_convention", "contract_revision"):
        _string(_field(value, key, path), path + "." + key)
    nu = _list(_field(value, "nu", path), path + ".nu")
    if len(nu) != 45:
        _fail(path + ".nu", "expected 45 coordinates")
    for i, coordinate in enumerate(nu):
        _integer(coordinate, f"{path}.nu[{i}]", maximum=1 if i < 43 else 4)
        if coordinate != 0:
            _fail(f"{path}.nu[{i}]", "label checkpoint requires nu = 0")


def validate(doc):
    """Check the complete prototype document, preserving opaque JSON metadata."""
    try:
        _check_json(doc)
    except RecursionError:
        _fail("$", "JSON nesting exceeds interpreter capacity")
    _mapping(doc, "$")
    if _field(doc, "format", "$") != SESSION_FORMAT:
        _fail("$.format", "unknown session format")
    _integer(_field(doc, "revision", "$"), "$.revision")
    _field(doc, "retained", "$")
    if "jumbling" not in doc:
        return
    section = _mapping(doc["jumbling"], "$.jumbling")
    if _field(section, "format", "$.jumbling") != JUMBLING_FORMAT:
        _fail("$.jumbling.format", "unknown jumbling format")
    episodes = _list(_field(section, "episodes", "$.jumbling"), "$.jumbling.episodes")
    for i, episode in enumerate(episodes):
        at = f"$.jumbling.episodes[{i}]"
        _mapping(episode, at)
        _identities(_field(episode, "identities", at), at + ".identities")
        start = _mapping(_field(episode, "start", at), at + ".start")
        kind = _field(start, "kind", at + ".start")
        if kind == "word":
            _retained_word(_field(start, "word", at + ".start"), at + ".start.word")
        elif kind == "exit":
            previous = _integer(_field(start, "episode", at + ".start"), at + ".start.episode")
            if previous >= i or episodes[previous]["end"] is None or (
                episodes[previous]["end"]["kind"] != "label"
            ):
                _fail(at + ".start.episode", "must name an earlier label exit")
            _retained_word(_field(start, "retained_moves", at + ".start"),
                           at + ".start.retained_moves")
        else:
            _fail(at + ".start.kind", "unknown episode start")
        _records(_field(episode, "journal", at), at + ".journal")
        state = _field(episode, "state", at)
        _state(state, at + ".state", revision=True)
        end = _field(episode, "end", at)
        if end is None:
            if i != len(episodes) - 1:
                _fail(at + ".end", "only the last episode may be open")
            continue
        _mapping(end, at + ".end")
        kind = _field(end, "kind", at + ".end")
        if kind == "witness":
            _retained_word(_field(end, "word", at + ".end"), at + ".end.word")
        elif kind == "label":
            certificate = _field(end, "certificate", at + ".end")
            _certificate(certificate, at + ".end.certificate")
            exit_state = _field(end, "exit_state", at + ".end")
            _state(exit_state, at + ".end.exit_state")
            if exit_state["digest"] != certificate["pose_digest"]:
                _fail(at + ".end.exit_state.digest", "differs from certificate pose_digest")
            if exit_state["digest"] != state["digest"]:
                _fail(at + ".end.exit_state.digest", "differs from episode's final state")
        else:
            _fail(at + ".end.kind", "unknown episode end")
    words = _list(_field(section, "words", "$.jumbling"), "$.jumbling.words")
    for i, word in enumerate(words):
        at = f"$.jumbling.words[{i}]"
        _mapping(word, at)
        if "name" in word:
            _string(word["name"], at + ".name")
        size = _integer(_field(word, "menu_size", at), at + ".menu_size", minimum=1)
        letters = decode_ids(_field(word, "letters", at), 600 * size, at + ".letters")
        _identities(_field(word, "identities", at), at + ".identities")
        _digest(_field(word, "recorded_at", at), at + ".recorded_at")
        if "parts" in word:
            _check_parts(word["parts"], letters, path=at + ".parts")
    scrambles = _list(_field(section, "scrambles", "$.jumbling"), "$.jumbling.scrambles")
    for i, scramble in enumerate(scrambles):
        at = f"$.jumbling.scrambles[{i}]"
        _mapping(scramble, at)
        _identities(_field(scramble, "identities", at), at + ".identities")
        _records(_field(scramble, "journal", at), at + ".journal")
        if "seed" in scramble and type(scramble["seed"]) is not int:
            _fail(at + ".seed", "seed must be an integer")


def dumps(doc):
    validate(doc)
    return (_json_text(doc) + "\n").encode("utf-8")


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            _fail(f"$.{key}", "duplicate JSON object key")
        result[key] = value
    return result


def _invalid_constant(value):
    _fail("$", "non-finite JSON number: " + value)


def loads(data):
    try:
        if isinstance(data, (bytes, bytearray)):
            data = data.decode("utf-8")
        doc = json.loads(data, parse_int=_parse_int, parse_constant=_invalid_constant,
                         object_pairs_hook=_unique_object)
    except (ValueError, TypeError, UnicodeError, RecursionError) as error:
        if isinstance(error, StoreFormatError):
            raise
        _fail("$", "invalid JSON: " + str(error))
    validate(doc)
    return doc


def _temp_prefix(path):
    return f".{path.name}.session-"


def _fsync_directory(directory):
    unsupported = {errno.EINVAL, errno.ENOTSUP, errno.EOPNOTSUPP, errno.ENOSYS}
    try:
        fd = os.open(directory, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    except OSError as error:
        if error.errno in unsupported or (os.name == "nt" and error.errno == errno.EACCES):
            return
        raise
    try:
        try:
            os.fsync(fd)
        except OSError as error:
            if error.errno not in unsupported:
                raise
    finally:
        os.close(fd)


def write_session(path, doc, *, crash_at=None):
    """Commit one complete revision; simulated crashes keep real temp files."""
    if crash_at not in (None, "after_temp_write", "before_replace", "after_replace"):
        raise ValueError("unknown crash point")
    data = dumps(doc)  # Validate before any filesystem mutation.
    path = Path(path)
    fd, temporary = tempfile.mkstemp(prefix=_temp_prefix(path), suffix=".tmp", dir=path.parent)
    temporary = Path(temporary)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
            if crash_at == "after_temp_write":
                raise SimulatedCrash(crash_at)
        if crash_at == "before_replace":
            raise SimulatedCrash(crash_at)
        os.replace(temporary, path)
        if crash_at == "after_replace":
            raise SimulatedCrash(crash_at)
        _fsync_directory(path.parent)
    except SimulatedCrash:
        raise
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def read_session(path):
    """Read only the committed file and discard, never promote, stale temps."""
    path = Path(path)
    try:
        return loads(path.read_bytes())
    finally:
        for stale in path.parent.iterdir():
            if (stale.name.startswith(_temp_prefix(path)) and stale.name.endswith(".tmp")
                    and stale.is_file() and not stale.is_symlink()):
                stale.unlink(missing_ok=True)


def cross_on(doc, start, identities, state):
    """Return a new revision with an open episode; no input is mutated."""
    validate(doc)
    result = copy.deepcopy(doc)
    section = result.setdefault("jumbling", {
        "format": JUMBLING_FORMAT, "episodes": [], "words": [], "scrambles": [],
    })
    if section["episodes"] and section["episodes"][-1]["end"] is None:
        _fail("$.jumbling.episodes", "an episode is already open")
    section["episodes"].append(copy.deepcopy({
        "identities": identities, "start": start, "journal": [], "state": state, "end": None,
    }))
    result["revision"] += 1
    validate(result)
    return result


def cross_off(doc, certificate, exit_state, labels_value):
    """Return one revision closing the episode and replacing retained labels."""
    validate(doc)
    result = copy.deepcopy(doc)
    episodes = result.get("jumbling", {}).get("episodes", [])
    if not episodes or episodes[-1]["end"] is not None:
        _fail("$.jumbling.episodes", "no open episode")
    episodes[-1]["end"] = copy.deepcopy({
        "kind": "label", "certificate": certificate, "exit_state": exit_state,
    })
    result["retained"] = copy.deepcopy(labels_value)
    result["revision"] += 1
    validate(result)
    return result
