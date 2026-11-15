"""Recompute p3 reference vectors in memory. Standard library only; writes nothing.

This is a format reference check, not an importer or an implementation of the
evidence/policy/engine contracts. All paths are relative to this repository.
"""
from __future__ import annotations

import array
import ast
import copy
import gzip
import hashlib
import io
import itertools
import json
import math
from pathlib import Path
import re
import struct
import sys
import zipfile
import zlib

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
SIZE = "PS1:resource_limit.persistence_size"
FORMAT = "PS1:input.persistence_format"
VERSION = "PS2:input.persistence_version"
CANON = "PS1:input.noncanonical_value"
DIGEST = "PS1:input.digest_mismatch"
PROVENANCE = "PS1:capability.provenance_incompatible"
REPLAY = "PS3:input.checkpoint_replay"
STATE = "EV4:input.recording_state"
UNTRANSLATABLE = "LB5:input.obligation_untranslatable"
DECIMAL = re.compile(r"(?:0|-?[1-9][0-9]*)\Z")
GZIP_HEADER = bytes.fromhex("1f8b08000000000000ff")
# FP.4: a record kind wholly or partly in the format freeze has an identifier without the draft suffix.
FROZEN_KINDS = frozenset({
    "word", "recipe", "macro", "macro-set", "protection-set", "binding", "key-map", "filter", "preferences",
    "checkpoint", "replay-path", "workspace", "evidence", "import-table", "migration", "exit", "void",
    "transition", "admission", "receipt", "workspace-inverse", "session-record", "publication",
    "research-bundle", "pick"})


class Refusal(ValueError):
    pass


def require(condition, code):
    if not condition:
        raise Refusal(code)


def equal(actual, expected, label):
    if actual != expected:
        raise AssertionError(f"{label}: {actual!r} != {expected!r}")


def outcome(operation):
    try:
        operation()
    except Refusal as exc:
        return str(exc)
    return "accepted"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def ident(kind):
    """FP.4's exact `format` for a c600 record kind."""
    return f"c600-{kind}" if kind in FROZEN_KINDS else f"c600-{kind}-draft"


# FP.10: the formats this reference reader dispatches. Old draft identifiers of frozen kinds are not among them.
KNOWN_FORMATS = frozenset(ident(kind) for kind in FROZEN_KINDS | {"playback"})


def supported(header):
    """FP.10: dispatch a record's or a Ref's format and version before any version-specific shape or Ref check."""
    require(isinstance(header, dict) and {"format", "version"} <= header.keys(), FORMAT)
    require(isinstance(header["format"], str) and header["format"] in KNOWN_FORMATS and
            type(header["version"]) is int and header["version"] == 1, VERSION)


def limits_from_text():
    text = (HERE / "README.md").read_bytes().decode("utf-8")
    rows = re.findall(r"^\| ([a-z][a-z0-9_]+) \| [^|]+ \| (\d+) \| (\d+) \|$", text, re.M)
    result = {name: (int(low), int(high)) for name, low, high in rows}
    equal(len(result), len(rows), "unique limit rows")
    require(bool(result), FORMAT)
    return result


LIMITS = limits_from_text()


def charge(row, value):
    low, high = LIMITS[row]
    require(type(value) is int and low <= value <= high, SIZE)


def canonical(value, profile="new"):
    def check(item, depth=0):
        if item is None or type(item) is bool:
            return
        if type(item) is int:
            require(profile != "new" or abs(item) <= 9007199254740991, CANON)
        elif isinstance(item, str):
            require(not any(0xD800 <= ord(c) <= 0xDFFF for c in item), CANON)
        elif isinstance(item, list):
            charge("json_depth", depth + 1)
            for child in item:
                check(child, depth + 1)
        elif isinstance(item, dict):
            charge("json_depth", depth + 1)
            for key, child in item.items():
                require(isinstance(key, str), CANON)
                check(key, depth + 1)
                check(child, depth + 1)
        else:
            raise Refusal(CANON)
    require(profile in {"new", "retained", "prototype"}, FORMAT)
    check(value)
    raw = json.dumps(value, ensure_ascii=profile == "retained", sort_keys=True,
                     separators=(",", ":"), allow_nan=False).encode("utf-8")
    return raw if profile == "retained" else raw + b"\n"


def read_canonical(raw):
    charge("record_decoded", len(raw))

    def pairs(items):
        obj = {}
        for key, value in items:
            require(key not in obj, CANON)
            obj[key] = value
        return obj

    def number(token):
        require(len(token.lstrip("-")) <= 16, CANON)
        return int(token)

    def no_float(_):
        raise Refusal(CANON)

    try:
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=pairs,
                           parse_int=number, parse_float=no_float, parse_constant=no_float)
        require(raw == canonical(value), CANON)
        return value
    except (UnicodeError, json.JSONDecodeError, RecursionError) as exc:
        raise Refusal(CANON) from exc


def int_value(text):
    require(isinstance(text, str) and DECIMAL.fullmatch(text) is not None, CANON)
    digits = text.lstrip("-")
    charge("integer_digits", len(digits))
    value = 0
    # Avoid the host's decimal-conversion limit without changing global settings.
    for start in range(0, len(digits), 9):
        chunk = digits[start:start + 9]
        value = value * 10 ** len(chunk) + int(chunk)
    return -value if text.startswith("-") else value


def int_text(value):
    if not value:
        return "0"
    negative = value < 0
    value = abs(value)
    chunks = []
    while value:
        value, tail = divmod(value, 1000000000)
        chunks.append(tail)
    result = str(chunks.pop()) + "".join(f"{part:09d}" for part in reversed(chunks))
    charge("integer_digits", len(result))
    return "-" + result if negative else result


def exact_value(value):
    require(isinstance(value, dict) and len(value) == 1, CANON)
    kind, data = next(iter(value.items()))
    if kind == "int":
        return int_value(data)
    if kind in {"rat", "q5"}:
        require(isinstance(data, list) and len(data) == (2 if kind == "rat" else 3), CANON)
        parts = tuple(int_value(item) for item in data)
        require(parts[-1] > 0 and math.gcd(*parts) == 1, CANON)
        return parts
    if kind == "f64":
        require(isinstance(data, str) and re.fullmatch(r"[0-9a-f]{16}", data) is not None, CANON)
        bits = int(data, 16)
        require((bits >> 52) & 2047 != 2047 and bits != 1 << 63, CANON)
        return struct.unpack(">d", bytes.fromhex(data))[0]
    raise Refusal(CANON)


def pack_gzip(raw):
    result = bytearray(GZIP_HEADER)
    for start in range(0, max(1, len(raw)), 65535):
        part = raw[start:start + 65535]
        result.append(int(start + len(part) == len(raw)))
        result.extend(struct.pack("<HH", len(part), len(part) ^ 65535))
        result.extend(part)
    result.extend(struct.pack("<II", zlib.crc32(raw), len(raw) & 0xFFFFFFFF))
    return bytes(result)


def unpack_gzip(packed):
    charge("record_stored", len(packed))
    require(packed[:10] == GZIP_HEADER, CANON)
    offset = 10
    out = bytearray()
    while True:
        require(offset + 5 <= len(packed) - 8, CANON)
        flag, count, inverse = struct.unpack_from("<BHH", packed, offset)
        require(flag in (0, 1) and inverse == count ^ 65535, CANON)
        require(flag == 1 or count == 65535, CANON)
        require(offset + 5 + count <= len(packed) - 8, CANON)
        charge("record_decoded", len(out) + count)
        out.extend(packed[offset + 5:offset + 5 + count])
        offset += 5 + count
        if flag:
            break
    require(offset + 8 == len(packed), CANON)
    require(packed[offset:] == struct.pack("<II", zlib.crc32(out), len(out) & 0xFFFFFFFF), DIGEST)
    require(pack_gzip(out) == packed, CANON)
    return bytes(out)


def record_header(record):
    supported(record)
    fields = {"format", "version", "identity", "payload", "provenance"}
    require(fields <= record.keys(), FORMAT)
    require(not (record.keys() - fields - {"extensions"}), FORMAT)
    names = []
    for extension in record.get("extensions", []):
        require(set(extension) == {"name", "required", "value"}, FORMAT)
        require(isinstance(extension["name"], str) and type(extension["required"]) is bool, FORMAT)
        require(not extension["required"], VERSION)
        names.append(extension["name"])
    require(names == sorted(set(names)), FORMAT)


def provenance(block):
    fields = {"block", "author", "producer", "profile", "depends", "parts", "pick", "declassified", "legacy"}
    require(isinstance(block, dict) and set(block) == fields, PROVENANCE)
    meta = block["block"]
    require(isinstance(meta, dict) and set(meta) == {"version", "form", "contract", "checker"}, PROVENANCE)
    require(type(meta["version"]) is int and meta["version"] == 1, PROVENANCE)
    require(meta["form"] in {"flow_static", "flow_runtime", "capability_separation"}, PROVENANCE)
    require(block["author"] in {"built_in", "own", "forked_built_in", "imported", "unknown"}, PROVENANCE)
    producer = block["producer"]
    kind = producer.get("kind")
    require(kind in {"manual", "import", "program"}, PROVENANCE)
    require(block["profile"] in {"none", "solving", "research", "tool_authoring"}, PROVENANCE)
    require((block["profile"] == "none") == (kind != "program"), PROVENANCE)
    if kind == "manual":
        require(set(producer) == {"kind"}, PROVENANCE)
    elif kind == "import":
        require(set(producer) == {"kind", "origin"}, PROVENANCE)
        require(block["legacy"] is not None and producer["origin"] == block["legacy"]["source"], PROVENANCE)
    else:
        require(set(producer) == {"kind", "module", "entry"}, PROVENANCE)
        require(meta["form"] != "flow_static" or meta["checker"] is not None, PROVENANCE)
    require(block["pick"] is None or (isinstance(block["pick"], dict) and set(block["pick"]) == {"record", "chosen"}),
            PROVENANCE)
    depends = block["depends"]
    if depends == {"kind": "not_recorded"}:
        require(kind == "import" or block["author"] == "unknown", PROVENANCE)
    elif meta["form"] == "capability_separation":
        require(depends in ({"read_state": "yes"}, {"read_state": "no"}), PROVENANCE)
    else:
        require(isinstance(depends, dict) and set(depends) == {"labels"}, PROVENANCE)
        labels = depends["labels"]
        require(isinstance(labels, list) and all(isinstance(label, str) for label in labels), PROVENANCE)
        require(labels == sorted(set(labels)) and set(labels) <= {"state", "computed", "research", "unknown"}, PROVENANCE)
    return block


def policy_use(block, form, available_contracts):
    provenance(block)
    require(block["block"]["form"] == form, PROVENANCE)
    require(block["block"]["contract"]["source"]["sha256"] in available_contracts, PROVENANCE)
    # This packet has no policy implementation or grant path.
    raise Refusal(PROVENANCE)


def encode_ia16(letters):
    raw = bytearray()
    for grip, item in letters:
        require(type(grip) is int and type(item) is int and 0 <= grip < 600 and 0 <= item < 60, CANON)
        raw.extend(struct.pack("<H", 60 * grip + item))
    return bytes(raw)


def decode_ia16(raw):
    require(len(raw) % 2 == 0, CANON)
    letters = []
    for (code,) in struct.iter_unpack("<H", raw):
        require(code < 36000, CANON)
        letters.append(divmod(code, 60))
    return letters


def word_chunks(raw):
    require(len(raw) % 2 == 0, CANON)
    size = 2 * LIMITS["ia16_chunk_letters"][1]
    return [raw[i:i + size] for i in range(0, len(raw), size)]


def read_chunks(chunks, length, digest, recorded_order, current_order):
    require(recorded_order == current_order, "KN1:input.identity_mismatch")
    for i, part in enumerate(chunks):
        charge("ia16_chunk_letters", len(part) // 2)
        require(i == len(chunks) - 1 or len(part) == 2 * LIMITS["ia16_chunk_letters"][1], CANON)
        decode_ia16(part)
    raw = b"".join(chunks)
    require(len(raw) == 2 * length, CANON)
    require(sha(raw) == digest, DIGEST)
    return decode_ia16(raw)


def npy_vector(raw):
    """Read only the pinned one-dimensional integer assets, without NumPy."""
    require(raw[:8] == b"\x93NUMPY\x01\x00", FORMAT)
    header_size = struct.unpack_from("<H", raw, 8)[0]
    header = ast.literal_eval(raw[10:10 + header_size].decode("ascii"))
    require(header["descr"] in {"<i4", "<u4"} and not header["fortran_order"], FORMAT)
    require(len(header["shape"]) == 1, FORMAT)
    data = raw[10 + header_size:]
    require(len(data) == 4 * header["shape"][0], FORMAT)
    values = array.array("i" if header["descr"] == "<i4" else "I")
    equal(values.itemsize, 4, "32-bit array carrier")
    values.frombytes(data)
    if sys.byteorder != "little":
        values.byteswap()
    return values


def i32_bytes(values):
    result = array.array("i", values)
    if sys.byteorder != "little":
        result.byteswap()
    return result.tobytes()


class RetainedReference:
    def __init__(self):
        self.manifest = json.loads((ROOT / "assets/manifest.json").read_bytes())
        for name in ("assets/primitives.npz", "assets/execution_trees.json.gz"):
            equal(sha((ROOT / name).read_bytes()), self.manifest["files"][name], name)
        with zipfile.ZipFile(ROOT / "assets/primitives.npz") as bundle:
            self.source = npy_vector(bundle.read("src.npy"))
            self.destination = npy_vector(bundle.read("dst.npy"))
            self.offsets = npy_vector(bundle.read("offsets.npy"))
        self.trees = json.loads(gzip.decompress((ROOT / "assets/execution_trees.json.gz").read_bytes()))
        equal(len(self.offsets), 1201, "all 1200 legal generators")
        self.count = self.manifest["mesh"]["slots"]
        equal(self.count, 259800, "all labelled slots")

    def expand(self, recipe):
        require(isinstance(recipe, list), FORMAT)
        charge("recipe_steps", len(recipe))
        normalized, moves = [], []
        for step in recipe:
            require(isinstance(step, dict), FORMAT)
            if step.get("kind") == "word":
                require(set(step) == {"kind", "moves"}, FORMAT)
                word = step["moves"]
                require(isinstance(word, list), FORMAT)
                charge("recipe_word_letters", len(word))
                require(all(type(m) is int and 1 <= abs(m) <= 1200 for m in word), CANON)
                normalized.append({"kind": "word", "moves": list(word)})
            elif step.get("kind") == "star":
                require(set(step) <= {"kind", "orbit", "node", "sign"}, FORMAT)
                orbit, node, sign = step.get("orbit"), step.get("node"), step.get("sign", 1)
                require(type(orbit) is int and 0 <= orbit < 35 and type(sign) is int and sign in (-1, 1), CANON)
                tree = self.trees[orbit]
                require(type(node) is int and 0 <= node < len(tree["parent"]), CANON)
                at, path = node, []
                while tree["parent"][at] >= 0:
                    path.append(tree["move"][at])
                    at = tree["parent"][at]
                path.reverse()
                inverse = lambda w: [-m for m in reversed(w)]
                word = inverse(path) + inverse(tree["relocation"]) + tree["seed"] + tree["relocation"] + path
                if sign < 0:
                    word = inverse(word)
                normalized.append({"kind": "star", "orbit": orbit, "node": node, "sign": sign})
            else:
                raise Refusal(FORMAT)
            charge("recipe_primitives", len(moves) + len(word))
            moves.extend(word)
        return normalized, moves

    def replay(self, moves):
        labels = array.array("i", range(self.count))
        prefixes = [sha(i32_bytes(labels))]
        for move in moves:
            first, last = self.offsets[abs(move) - 1:abs(move) + 1]
            source = self.source[first:last]
            destination = self.destination[first:last]
            if move < 0:
                source, destination = destination, source
            saved = [labels[pos] for pos in source]
            for dest, label in zip(destination, saved):
                labels[dest] = label
            prefixes.append(sha(i32_bytes(labels)))
        return labels, prefixes


def conversion_result(reference, recipe):
    normalized, old_moves = reference.expand(recipe)
    payload = {"steps": normalized, "primitive_count": int_text(len(old_moves)),
               "expansion": {"role": "recipe_expansion", "sha256": sha(i32_bytes(old_moves))},
               "contract": {"version": "retained-0.4", "source": digest_ref("source", (ROOT / "core.py").read_bytes())}}
    wire = {"format": ident("recipe"), "version": 1,
            "identity": {"puzzle": "full", "model_id": reference.manifest["model_id"],
                         "menu_id": None, "menu_order": None, "contract_rev": None},
            "payload": payload, "provenance": None}
    decoded = read_canonical(canonical(wire))
    record_header(decoded)
    restored = decoded["payload"]
    normalized_again, new_moves = reference.expand(restored["steps"])
    equal(normalized_again, normalized, "retained recipe normalization")
    equal(new_moves, old_moves, "literal primitive conversion")
    equal(int_value(restored["primitive_count"]), len(new_moves), "converted count")
    before, prefixes = reference.replay(old_moves)
    after, new_prefixes = reference.replay(new_moves)
    equal(before, after, "complete converted labels")
    equal(prefixes, new_prefixes, "every converted prefix")
    net = sorted((label, position) for position, label in enumerate(after) if label != position)
    sources = [s for s, _ in net]
    destinations = [d for _, d in net]
    net_bytes = i32_bytes(sources) + i32_bytes(destinations)
    # Apply the composed source-to-destination mapping once and compare full state.
    once = array.array("i", range(reference.count))
    for source, destination in net:
        once[destination] = source
    equal(once, after, "source-to-destination net replay")
    return {"primitive_count": len(new_moves), "expansion_sha256": sha(i32_bytes(new_moves)),
            "pre": prefixes[0], "post": prefixes[-1], "prefixes_sha256": sha(canonical(prefixes)),
            "net_sha256": sha(net_bytes), "moved_slots": len(net), "labels": len(after),
            "source_sha256": sha(canonical(recipe, "retained")), "content_sha256": sha(canonical(wire))}


def digest_ref(role, data):
    return {"role": role, "sha256": sha(data)}


def synthetic_ref(kind, name):
    return {"format": ident(kind), "version": 1,
            "content": digest_ref("content", canonical({"fixture": name}))}


def signed_word(record, resolve, reference):
    """FP.13's retained word, including the empty form outside legacy recipes."""
    payload = record["payload"]
    require(payload.get("alphabet") == "signed_generators", FORMAT)
    if "recipe" not in payload:
        require(set(payload) == {"alphabet", "start", "length", "letters"}, FORMAT)
        require(payload["length"] == "0" and payload["letters"] == digest_ref("recipe_expansion", b""), CANON)
        return []
    require(set(payload) == {"alphabet", "start", "recipe"}, FORMAT)
    recipe = resolve(payload["recipe"])
    _, moves = reference.expand(recipe["steps"])
    require(recipe["primitive_count"] == str(len(moves)) and
            recipe["expansion"] == digest_ref("recipe_expansion", i32_bytes(moves)), DIGEST)
    return moves


def crossing_protection(current, before, after, reverse):
    """Only the crossing's recorded level delta applies to current protection."""
    result = copy.deepcopy(current)
    if before["level"] != after["level"]:
        source, target = (after, before) if reverse else (before, after)
        require(current["level"] == source["level"], REPLAY)
        result["level"] = target["level"]
    return result


def durable_result(given, reference=None):
    """Synthetic H11 records: round-trip then recover through content references."""
    labels = digest_ref("labels", i32_bytes(array.array("i", range(259800))))
    poses = digest_ref("poses", b"synthetic exit X poses")
    identity = {"puzzle": "jumbling", "model_id": "fixture-model", "menu_id": "fixture-menu",
                "menu_order": "fixture-order", "contract_rev": "fixture-1"}
    objects, blobs = {}, {}
    pose_bytes = {poses["sha256"]: b"synthetic exit X poses"}
    fixture_provenance = json.loads((HERE / "examples.json").read_bytes())["records"]["manual_flow"]["provenance"]

    def keep(kind, payload):
        record = {"format": ident(kind), "version": 1, "identity": identity,
                  "provenance": fixture_provenance if kind in {"workspace-inverse", "word", "protection-set"} else None, "payload": payload}
        raw = canonical(record)
        ref = {"format": record["format"], "version": 1, "content": digest_ref("content", raw)}
        objects[ref["content"]["sha256"]] = record
        return ref

    def keep_blob(raw):
        blobs[sha(raw)] = raw.hex()
        return blob(raw)

    def keep_pose(raw):
        pose = digest_ref("poses", raw)
        pose_bytes[pose["sha256"]] = raw
        return pose

    def protection(value):
        if isinstance(value, str):
            value = {"level": "poses" if value == "A" else "labels",
                     "members": [{"kind": "orbit", "id": int(value == "C")}], "mode": "net",
                     "contract": {"version": "fixture-1", "source": digest_ref("source", b"protection")},
                     "origin": None}
        return keep("protection-set", value)

    def cross(ctx, original, reverse):
        if original["action"] in {"exit", "entry"}:
            records = [objects[ref["content"]["sha256"]]["payload"] for ref in
                       (ctx["protection"], original["context_before"]["protection"], original["context_after"]["protection"])]
            ctx["protection"] = protection(crossing_protection(*records, reverse))

    def position(head, on=False, episode=None):
        return {"head": head, "mode": "jumble_on" if on else "jumble_off",
                "episode": episode or "episode-X" if on else None,
                "state": poses if on else labels}

    def context(preferences, protection_value):
        return {"preferences": synthetic_ref("preferences", preferences),
                "protection": protection(protection_value)}

    def revision(pos, state, workspace, protection):
        return {"session": "fixture-session", "epoch": "epoch-1", "identity": identity,
                "mode": pos["mode"], "head": pos["head"], "state": str(state),
                "workspace": str(workspace), "protection": str(protection), "digest": pos["state"]}

    root = position("P", True)
    current, ctx = root, context("A", "A")
    rev = revision(current, 0, 0, 0)
    events = {"P": {"parent": None, "kind": "pose"},
              "X": {"parent": "P", "kind": "exit"},
              "M": {"parent": "X", "kind": "retained_move"},
              "N": {"parent": "X", "kind": "retained_move"},
              "Y": {"parent": "P", "kind": "exit"}}
    transactions, admissions, rows, row_refs, exits = [], [], [], [], {}
    previous = None
    pending_rollback = []
    for action in given["actions"]:
        kind = action["action"]
        tid = str(len(transactions))
        command = action.get("command", "C" + str(len(admissions)))
        request_bytes = canonical({"row": kind, "args": action, "guards": {}, "identities": [],
                                   "budget": None, "grants": [], "token_binding": None})
        request = digest_ref("content", request_bytes)
        admission = keep("admission", {"command": command, "request": request, "base": rev,
                                      "session": rev["session"], "epoch": rev["epoch"], "row": kind,
                                      "input": keep_blob(request_bytes)})
        admissions.append(admission)
        if kind == "rollback":
            pending_rollback.append((admission, action.get("proved", True)))
            continue
        next_ctx = copy.deepcopy(ctx)
        target = position(action.get("head", current["head"]), action.get("head") == "P")
        event = {"kind": "journal", "id": action.get("head"), "target": None, "path": []}
        entrance, exit_ref = None, None
        workspace = kind == "workspace"
        if kind == "workspace":
            target = current
            for field in ("preferences", "protection"):
                if field in action:
                    next_ctx[field] = (synthetic_ref("preferences", action[field]) if field == "preferences"
                                       else protection(action[field]))
            changed = [{"before": ctx[field], "after": next_ctx[field]} for field in ctx if ctx[field] != next_ctx[field]]
            if "checkpoint" in action:
                # A non-undoable commit row with a non-puzzle effect: no inverse (LI.9, JI.27).
                changed.append({"before": None, "after": synthetic_ref("checkpoint", action["checkpoint"])})
                inverse = None
            else:
                inverse = keep("workspace-inverse", {"changed": [{"before": c["after"], "after": c["before"]} for c in changed]})
            event = {"kind": "workspace", "target": None, "inverse": inverse, "changed": changed}
        elif kind in {"undo", "redo"}:
            index = action.get("target")
            if index is None:
                # Legacy vectors name the recorded target by its resulting head.
                side = "before" if kind == "undo" else "after"
                index = next(i for i in reversed(range(len(rows)))
                             if rows[i]["action"] not in {"undo", "redo", "branch"}
                             and rows[i][side]["head"] == action["head"])
            original = rows[index]
            require(original["event"]["kind"] != "workspace" or original["event"]["inverse"] is not None,
                    "KN5:stale.undo_superseded")
            event["target"] = row_refs[index]
            target = original["before" if kind == "undo" else "after"]
            workspace = original["event"]["kind"] == "workspace"
            source_ctx = original["context_after" if kind == "undo" else "context_before"]
            dest_ctx = original["context_before" if kind == "undo" else "context_after"]
            if workspace:
                event = copy.deepcopy(original["event"])
                event["target"] = row_refs[index]
                if kind == "undo":
                    event["changed"] = [{"before": c["after"], "after": c["before"]} for c in event["changed"]]
                event["inverse"] = keep("workspace-inverse", {"changed": [{"before": c["after"], "after": c["before"]} for c in event["changed"]]})
                target = current
            if workspace:
                for field in ctx:
                    if source_ctx[field] != dest_ctx[field]:
                        next_ctx[field] = dest_ctx[field]
            else:
                cross(next_ctx, original, kind == "undo")
        elif kind == "branch" and "path" in action:
            target = current
            for step in action["path"]:
                original = rows[step["target"]]
                reverse = step["direction"] == "undo"
                require(target == original["after" if reverse else "before"], REPLAY)
                target = original["before" if reverse else "after"]
                cross(next_ctx, original, reverse)
                event["path"].append({"direction": step["direction"], "transition": row_refs[step["target"]]})
        elif kind == "entry":
            binding = action["entrance"]
            entry_pose = poses
            if binding == "exit_root":
                head, suffix, seen = current["head"], [], set()
                while head not in exits:
                    require(head not in seen, REPLAY)
                    seen.add(head)
                    move = events.get(head)
                    require(move is not None and move["kind"] == "retained_move", REPLAY)
                    suffix.append(synthetic_ref("word", head))
                    head = move["parent"]
                suffix.reverse()
                exit_record = objects[exits[head]["content"]["sha256"]]["payload"]
                entry_pose = exit_record["pose_digest"]
                if suffix:
                    # Synthetic replay pins the exit pose and every retained word in order.
                    entry_pose = keep_pose(canonical({"before": entry_pose, "suffix": suffix}))
                entrance = {"kind": binding, "exit": exits[head], "crossing": head,
                            "suffix": digest_ref("content", canonical(suffix))}
            else:
                witness = {"alphabet": "signed_generators", "start": labels}
                if action.get("empty_witness"):
                    witness.update({"length": "0", "letters": digest_ref("recipe_expansion", b"")})
                else:
                    witness["recipe"] = keep("recipe", {"steps": [{"kind": "word", "moves": [1, -1]}],
                        "primitive_count": "2", "expansion": digest_ref("recipe_expansion", i32_bytes([1, -1])),
                        "contract": {"version": "fixture-1", "source": digest_ref("source", b"retained")}})
                entrance = {"kind": "solved_witness", "word": keep("word", witness)}
            target = position(action["head"], True, "episode-entry")
            target["state"] = entry_pose
        elif kind == "jumble_move":
            require(current["mode"] == "jumble_on", REPLAY)
            target = position(action["head"], True, current["episode"])
            letters = action["letters"]
            target["state"] = keep_pose(canonical({"before": current["state"], "letters": letters}))
            events[action["head"]] = {"parent": current["head"], "kind": kind, "letters": letters}
        elif kind == "retained_move":
            require(current["mode"] == "jumble_off", REPLAY)
            events[action["head"]] = {"parent": current["head"], "kind": kind}
        if "crossing_protection" in action:
            require(kind in {"exit", "entry"}, REPLAY)
            next_ctx["protection"] = protection(action["crossing_protection"])
        if kind == "exit":
            exit_ref = keep("exit", {"id": action["head"], "episode": current["episode"], "crossing": action["head"],
                                    "before": current, "after": target, "pose_digest": current["state"], "labels": labels,
                                    "poses": keep_blob(pose_bytes[current["state"]["sha256"]]),
                                    "certificate": synthetic_ref("evidence", "fixture-certificate"),
                                    "contract": {"version": "fixture-1", "source": digest_ref("source", b"H11")}})
            exits[action["head"]] = exit_ref
            events[action["head"]] = {"parent": current["head"], "kind": kind}
        voids = [keep("void", {"exit": ref, "transition": tid, "reason": kind}) for ref in exits.values()] if kind in {"reset", "import", "restore"} else []
        next_rev = revision(target, int(rev["state"]) + int(not workspace),
                            int(rev["workspace"]) + int(workspace or next_ctx != ctx),
                            int(rev["protection"]) + int(next_ctx["protection"] != ctx["protection"]))
        status = ({"kind": "completed", "completion": {"kind": "workspace", "revision": next_rev, "result": None}}
                  if kind == "workspace" and "checkpoint" not in action else {"kind": "committed", "record": {"event": tid, "pre": current["state"],
                                                                                 "post": target["state"], "revision": next_rev}})
        receipt = keep("receipt", {"command": command, "request": request, "job": None, "status": status})
        row = {"id": tid, "sequence": tid, "previous": previous, "action": kind,
               "before": current, "after": target, "context_before": ctx, "context_after": next_ctx,
               "event": event, "exit": exit_ref, "entrance": entrance, "voids": voids,
               "command": command, "request": request, "admission": admission, "receipt": receipt, "revision": next_rev}
        row_ref = keep("transition", row)
        rows.append(row)
        row_refs.append(row_ref)
        transactions.append({"committed": action.get("committed", True), "transition": row_ref})
        current, ctx, rev, previous = target, next_ctx, next_rev, row_ref
    durable = read_canonical(canonical({"objects": objects, "blobs": blobs, "transactions": transactions, "admissions": admissions,
                                       "root": root, "context": context("A", "A"), "events": events}))
    if given.get("corrupt") or given.get("corrupt_admission") or given.get("drop_witness"):
        # Mutate a stored reference, not the constructor's live objects.
        last = durable["transactions"][-1]["transition"]["content"]["sha256"]
        changed = copy.deepcopy(durable["objects"][last])
        payload = changed["payload"]
        if given.get("drop_witness"):
            del durable["objects"][payload["entrance"]["word"]["content"]["sha256"]]
        elif given.get("corrupt_admission"):
            old_ref = payload["admission"]
            admission = copy.deepcopy(durable["objects"][old_ref["content"]["sha256"]])
            mutation = given["corrupt_admission"]
            if mutation["field"].startswith("input."):
                raw = bytes.fromhex(durable["blobs"][admission["payload"]["input"]["stored"]["sha256"]])
                decoded = read_canonical(raw)
                target = decoded
                *parents, leaf = mutation["field"].split(".")[1:]
                for part in parents:
                    target = target[part]
                target[leaf] = mutation["value"]
                raw = canonical(decoded)
                durable["blobs"][sha(raw)] = raw.hex()
                admission["payload"]["input"] = blob(raw)
            else:
                target = admission["payload"]
                *parents, leaf = mutation["field"].split(".")
                for part in parents:
                    target = target[part]
                target[leaf] = mutation["value"]
            if mutation.get("rehash_request"):
                admission["payload"]["request"] = digest_ref("content", raw)
                payload["request"] = admission["payload"]["request"]
                receipt_ref = payload["receipt"]
                receipt = copy.deepcopy(durable["objects"][receipt_ref["content"]["sha256"]])
                receipt["payload"]["request"] = payload["request"]
                payload["receipt"] = {**receipt_ref, "content": digest_ref("content", canonical(receipt))}
                durable["objects"][payload["receipt"]["content"]["sha256"]] = receipt
            new_ref = {**old_ref, "content": digest_ref("content", canonical(admission))}
            durable["objects"][new_ref["content"]["sha256"]] = admission
            durable["admissions"] = [new_ref if ref == old_ref else ref for ref in durable["admissions"]]
            payload["admission"] = new_ref
        else:
            *parents, leaf = given["corrupt"].split(".")
            for part in parents:
                payload = payload[part]
            payload[leaf] = None
        changed_digest = sha(canonical(changed))
        durable["objects"][changed_digest] = changed
        durable["transactions"][-1]["transition"]["content"]["sha256"] = changed_digest
    if given.get("corrupt_event"):
        mutation = given["corrupt_event"]
        durable["events"][mutation["head"]]["kind"] = mutation["kind"]
    if (given.get("corrupt_exit") or given.get("drop_exit_blob") or given.get("future_exit_version")
            or given.get("corrupt_after_state") or given.get("corrupt_retained_parent") or given.get("corrupt_revision")):
        # Rebind content and terminal references so these controls reach the semantic checks.
        row_ref = durable["transactions"][-1]["transition"]
        record = copy.deepcopy(durable["objects"][row_ref["content"]["sha256"]])
        payload = record["payload"]
        if given.get("corrupt_revision"):
            mutation = given["corrupt_revision"]
            payload["revision"][mutation["field"]] = mutation["value"]
            receipt_ref = payload["receipt"]
            receipt = copy.deepcopy(durable["objects"][receipt_ref["content"]["sha256"]])
            status = receipt["payload"]["status"]
            status.get("record", status.get("completion"))["revision"] = payload["revision"]
            payload["receipt"] = {**receipt_ref, "content": digest_ref("content", canonical(receipt))}
            durable["objects"][payload["receipt"]["content"]["sha256"]] = receipt
        elif given.get("corrupt_after_state") or given.get("corrupt_retained_parent"):
            if given.get("corrupt_retained_parent"):
                mutation = given["corrupt_retained_parent"]
                durable["events"][mutation["head"]]["parent"] = mutation["parent"]
                suffix = [synthetic_ref("word", head) for head in mutation["suffix"]]
                binding = payload["entrance"]
                binding["suffix"] = digest_ref("content", canonical(suffix))
                exit_record = durable["objects"][binding["exit"]["content"]["sha256"]]["payload"]
                payload["after"]["state"] = digest_ref("poses", canonical({"before": exit_record["pose_digest"], "suffix": suffix}))
            else:
                payload["after"]["state"] = given["corrupt_after_state"]
            payload["revision"]["digest"] = payload["after"]["state"]
            receipt_ref = payload["receipt"]
            receipt = copy.deepcopy(durable["objects"][receipt_ref["content"]["sha256"]])
            receipt["payload"]["status"]["record"]["post"] = payload["after"]["state"]
            receipt["payload"]["status"]["record"]["revision"] = payload["revision"]
            payload["receipt"] = {**receipt_ref, "content": digest_ref("content", canonical(receipt))}
            durable["objects"][payload["receipt"]["content"]["sha256"]] = receipt
        else:
            exit_ref = payload["exit"]
            exit_record = copy.deepcopy(durable["objects"][exit_ref["content"]["sha256"]])
            if given.get("drop_exit_blob"):
                durable["blobs"].pop(exit_record["payload"]["poses"]["stored"]["sha256"], None)
            elif given.get("future_exit_version"):
                # FP.10: the exit Ref and its record both declare an unknown version.
                exit_record["version"] = 2
                exit_ref = {**exit_ref, "version": 2}
            else:
                mutation = given["corrupt_exit"]
                target = exit_record["payload"]
                *parents, leaf = mutation["field"].split(".")
                for part in parents:
                    target = target[part]
                target[leaf] = mutation["value"]
                if "blob_hex" in mutation:
                    raw = bytes.fromhex(mutation["blob_hex"])
                    durable["blobs"][sha(raw)] = raw.hex()
            payload["exit"] = {**exit_ref, "content": digest_ref("content", canonical(exit_record))}
            durable["objects"][payload["exit"]["content"]["sha256"]] = exit_record
        durable["objects"][sha(canonical(record))] = record
        row_ref["content"] = digest_ref("content", canonical(record))

    def resolve(ref):
        require(ref is not None, REPLAY)
        supported(ref)
        record = durable["objects"].get(ref["content"]["sha256"])
        require(record is not None and digest_ref("content", canonical(record)) == ref["content"], REPLAY)
        supported(record)
        require(record["format"] == ref["format"] and record["version"] == ref["version"], REPLAY)
        return record["payload"]

    def validate_admission(admission, base):
        # These rows are the synthetic fixture vocabulary, not product command schemas.
        require(admission["base"] == base and admission["session"] == base["session"] and
                admission["epoch"] == base["epoch"], REPLAY)
        ref = admission["input"]
        require(isinstance(ref, dict), REPLAY)
        raw = durable["blobs"].get(ref["stored"]["sha256"])
        require(raw is not None, REPLAY)
        raw = bytes.fromhex(raw)
        require(blob(raw) == ref and digest_ref("content", raw) == admission["request"], REPLAY)
        try:
            request = read_canonical(raw)
        except Refusal as exc:
            raise Refusal(REPLAY) from exc
        require(isinstance(request, dict) and
                set(request) == {"row", "args", "guards", "identities", "budget", "grants", "token_binding"}, REPLAY)
        require(isinstance(request["row"], str) and request["row"] == admission["row"] and
                isinstance(request["args"], dict) and isinstance(request["guards"], dict) and
                isinstance(request["identities"], list) and isinstance(request["grants"], list) and
                (request["budget"] is None or isinstance(request["budget"], dict)) and
                (request["token_binding"] is None or isinstance(request["token_binding"], dict)), REPLAY)
        args, kind = request["args"], request["row"]
        require(args.get("action") == kind and kind in {"exit", "entry", "retained_move", "jumble_move",
                "undo", "redo", "branch", "reset", "import", "restore", "workspace", "rollback"}, REPLAY)
        if kind in {"exit", "entry", "retained_move", "jumble_move", "branch", "reset", "import", "restore"}:
            require(isinstance(args.get("head"), str), REPLAY)
        if kind == "entry":
            require(args.get("entrance") in {"solved_witness", "exit_root"}, REPLAY)
        elif kind == "jumble_move":
            require(isinstance(args.get("letters"), list) and bool(args["letters"]), REPLAY)
        elif kind in {"undo", "redo"}:
            require((type(args["target"]) is int and args["target"] >= 0) if "target" in args
                    else isinstance(args.get("head"), str), REPLAY)
        elif kind == "workspace":
            require(any(field in args for field in ("preferences", "protection", "checkpoint")), REPLAY)
        return request

    current, ctx = durable["root"], durable["context"]
    rev, previous, committed = revision(current, 0, 0, 0), None, 0
    recovered_revisions = [rev]
    recovered_refs = []

    def recovered_target(index):
        require(type(index) is int and 0 <= index < len(recovered_refs), REPLAY)
        return recovered_refs[index]

    terminals, found_exits, voided = {}, {}, set()
    last_entrance, witness_length = None, None
    pose_roots = {}
    for transaction in durable["transactions"]:
        if not transaction["committed"]:
            break
        ref = transaction["transition"]
        row = resolve(ref)
        require(row["before"] == current and row["context_before"] == ctx and row["previous"] == previous
                and row["sequence"] == str(committed), REPLAY)
        admission, receipt = resolve(row["admission"]), resolve(row["receipt"])
        require(row["admission"] in durable["admissions"], REPLAY)
        request = validate_admission(admission, rev)
        require(request["row"] == row["action"], REPLAY)
        args, kind = request["args"], row["action"]
        if "head" in args and kind not in {"undo", "redo"}:
            require(args["head"] == row["after"]["head"], REPLAY)
        original = resolve(row["event"]["target"]) if row["event"]["target"] else row
        if kind in {"undo", "redo"}:
            require(row["event"]["target"] in recovered_refs, REPLAY)
            side = original["before" if kind == "undo" else "after"]
            if "target" in args:
                require(recovered_target(args["target"]) == row["event"]["target"], REPLAY)
            if "head" in args:
                require(args["head"] == side["head"], REPLAY)
            require(row["event"]["kind"] == original["event"]["kind"], REPLAY)
            if row["event"]["kind"] == "journal":
                require(row["after"] == side, REPLAY)
        elif kind == "branch":
            path = args.get("path", [])
            require(isinstance(path, list) and len(path) == len(row["event"]["path"]), REPLAY)
            pos = current
            for requested, recorded in zip(path, row["event"]["path"]):
                require(isinstance(requested, dict) and requested.get("direction") in {"undo", "redo"}, REPLAY)
                target_ref = recovered_target(requested.get("target"))
                require(recorded == {"direction": requested["direction"], "transition": target_ref}, REPLAY)
                crossed = resolve(target_ref)
                reverse = requested["direction"] == "undo"
                require(pos == crossed["after" if reverse else "before"], REPLAY)
                pos = crossed["before" if reverse else "after"]
            if "path" in args:
                require(pos == row["after"], REPLAY)
            # Older head-only fixtures select predefined branches without a path.
        elif kind == "entry":
            require(row["entrance"] is not None and args["entrance"] == row["entrance"]["kind"], REPLAY)
        elif kind == "workspace":
            expected_ctx = copy.deepcopy(ctx)
            for field in ("preferences", "protection"):
                if field in args:
                    expected_ctx[field] = (synthetic_ref("preferences", args[field]) if field == "preferences"
                                           else protection(args[field]))
            changed = [{"before": ctx[field], "after": expected_ctx[field]} for field in ctx if ctx[field] != expected_ctx[field]]
            if "checkpoint" in args:
                changed.append({"before": None, "after": synthetic_ref("checkpoint", args["checkpoint"])})
            require(row["context_after"] == expected_ctx and row["event"]["changed"] == changed, REPLAY)
        if kind in {"exit", "entry"} and "crossing_protection" in args:
            require(protection(args["crossing_protection"]) == row["context_after"]["protection"], REPLAY)
        require(row["command"] == admission["command"] == receipt["command"] and
                row["request"] == admission["request"] == receipt["request"], REPLAY)
        result = receipt["status"].get("record", receipt["status"].get("completion"))
        require(result["revision"] == row["revision"], REPLAY)
        is_workspace = row["event"]["kind"] == "workspace"
        require(row["revision"] == revision(row["after"], int(rev["state"]) + int(not is_workspace),
                                            int(rev["workspace"]) + int(is_workspace or row["context_after"] != ctx),
                                            int(rev["protection"]) + int(row["context_after"]["protection"] != ctx["protection"])),
                REPLAY)
        if receipt["status"]["kind"] == "committed":
            require(result["pre"] == row["before"]["state"] and result["post"] == row["after"]["state"], REPLAY)
        if row["event"]["kind"] == "workspace":
            require(row["before"] == row["after"] and row["revision"]["state"] == rev["state"], REPLAY)
            if row["event"]["inverse"] is None:
                require(row["action"] == "workspace" and receipt["status"]["kind"] == "committed"
                        and result["pre"] == result["post"], REPLAY)
            else:
                require(resolve(row["event"]["inverse"])["changed"] ==
                        [{"before": c["after"], "after": c["before"]} for c in row["event"]["changed"]], REPLAY)
        else:
            require(int(row["revision"]["state"]) == int(rev["state"]) + 1, REPLAY)
        require((row["exit"] is not None) == (row["action"] == "exit"), REPLAY)
        if row["action"] in {"retained_move", "jumble_move", "exit"}:
            move = durable["events"].get(row["after"]["head"])
            require(move is not None and move["kind"] == row["action"] and
                    move["parent"] == row["before"]["head"], REPLAY)
        if row["exit"]:
            exit_record = resolve(row["exit"])
            require(exit_record["before"] == row["before"] and
                    exit_record["after"] == row["after"] and exit_record["pose_digest"] == row["before"]["state"], REPLAY)
            pose_blob = exit_record["poses"]
            require(isinstance(pose_blob, dict), REPLAY)
            raw = durable["blobs"].get(pose_blob["stored"]["sha256"])
            require(raw is not None, REPLAY)
            raw = bytes.fromhex(raw)
            require(blob(raw) == pose_blob and digest_ref("poses", raw) == exit_record["pose_digest"], REPLAY)
            found_exits[row["exit"]["content"]["sha256"]] = exit_record
        if row["entrance"]:
            last_entrance = row["entrance"]["kind"]
        if row["action"] in {"retained_move", "jumble_move"}:
            mode, role = ("jumble_on", "poses") if row["action"] == "jumble_move" else ("jumble_off", "labels")
            require(row["before"]["mode"] == row["after"]["mode"] == mode and
                    row["before"]["episode"] == row["after"]["episode"] and row["after"]["state"]["role"] == role and
                    row["context_after"] == ctx, REPLAY)
            if row["action"] == "jumble_move":
                require(move["letters"] == request["args"]["letters"] and
                        row["after"]["state"] == digest_ref("poses", canonical({"before": row["before"]["state"],
                                                                               "letters": move["letters"]})), REPLAY)
                pose_roots[row["after"]["head"]] = pose_roots.get(row["before"]["head"])
        if row["event"]["kind"] == "journal" and row["action"] in {"undo", "redo", "branch"}:
            expected_protection = resolve(ctx["protection"])
            steps = row["event"]["path"] if row["action"] == "branch" else [{"direction": row["action"], "transition": row["event"]["target"]}]
            for step in steps:
                crossed = resolve(step["transition"])
                if crossed["action"] in {"exit", "entry"}:
                    expected_protection = crossing_protection(expected_protection,
                        resolve(crossed["context_before"]["protection"]), resolve(crossed["context_after"]["protection"]),
                        step["direction"] == "undo")
            require(resolve(row["context_after"]["protection"]) == expected_protection and
                    row["context_after"]["preferences"] == ctx["preferences"], REPLAY)
        binding = original["entrance"]
        if row["action"] in {"entry", "redo"} and binding and binding["kind"] == "solved_witness":
            pose_roots[row["after"]["head"]] = None
            try:
                word = resolve(binding["word"])
                record = durable["objects"][binding["word"]["content"]["sha256"]]
                require(record["format"] == ident("word") and record["identity"] == identity, REPLAY)
                reference = reference or RetainedReference()
                moves = signed_word(record, resolve, reference)
                _, prefixes = reference.replay(moves)
                require(word["start"] == labels, REPLAY)
                require(prefixes[-1] == original["before"]["state"]["sha256"], REPLAY)
                # Only these two solved-pose fixtures are modelled; this is no jumbling engine.
                require(moves in ([], [1, -1]) and original["after"]["state"] == poses, REPLAY)
                witness_length = len(moves)
            except Refusal as exc:
                raise Refusal(REPLAY) from exc
        if row["action"] in {"entry", "redo"} and binding and binding["kind"] == "exit_root":
            require(binding["exit"]["content"]["sha256"] not in voided, "KN5:input.exit_record_voided")
            require(binding["exit"]["content"]["sha256"] in found_exits, REPLAY)
            exit_record = resolve(binding["exit"])
            require(binding["crossing"] == exit_record["crossing"], REPLAY)
            head, suffix, seen = original["before"]["head"], [], set()
            while head != binding["crossing"]:
                require(head not in seen, REPLAY)
                seen.add(head)
                event = durable["events"].get(head)
                require(event is not None and event["kind"] == "retained_move", REPLAY)
                suffix.append(synthetic_ref("word", head))
                head = event["parent"]
            suffix.reverse()
            require(binding["suffix"] == digest_ref("content", canonical(suffix)), REPLAY)
            entry_root = exit_record["pose_digest"]
            replayed = (digest_ref("poses", canonical({"before": entry_root, "suffix": suffix}))
                        if suffix else entry_root)
            require(replayed == original["after"]["state"], REPLAY)
            pose_roots[row["after"]["head"]] = entry_root
        for void_ref in row["voids"]:
            fact = resolve(void_ref)
            require(fact["transition"] == row["id"], REPLAY)
            voided.add(fact["exit"]["content"]["sha256"])
        terminals[row["command"]] = receipt
        current, ctx, rev, previous = row["after"], row["context_after"], row["revision"], ref
        recovered_revisions.append(rev)
        recovered_refs.append(ref)
        committed += 1
    for admission_ref, proved in pending_rollback:
        admission = resolve(admission_ref)
        require(admission["base"] in recovered_revisions, REPLAY)
        validate_admission(admission, admission["base"])
        terminals[admission["command"]] = {"command": admission["command"], "request": admission["request"], "job": None,
                                           "status": {"kind": "failed", "error": {"code": "KN5:resource_limit.engine_lost"}} if proved else {"kind": "unknown_outcome"}}
    valid = None
    if current["mode"] == "jumble_off":
        for digest, record in found_exits.items():
            if digest in voided:
                continue
            head, seen = current["head"], set()
            while head != record["crossing"]:
                require(head not in seen, REPLAY)
                seen.add(head)
                event = durable["events"].get(head)
                if event is None or event["kind"] != "retained_move":
                    break
                head = event["parent"]
            if head == record["crossing"]:
                valid = record
    result = {"head": current["head"], "mode": current["mode"], "valid_exit": valid["id"] if valid else None,
              "pose_root": valid["pose_digest"]["sha256"] if valid else None, "voided": bool(voided),
              "committed_transactions": committed,
              "resume": "already_on" if current["mode"] == "jumble_on" else "replay_from_exit" if valid else "KN5:input.jumble_on_labels_only"}
    if "observe" in given:
        if current["mode"] == "jumble_on":
            entry_root = pose_roots.get(current["head"])
            result["pose_root"] = entry_root["sha256"] if entry_root else None
        def name(field):
            return next((n for n in ("A", "B", "C") if ctx[field] ==
                         (synthetic_ref("preferences", n) if field == "preferences" else protection(n))), None)
        result.update({"preferences": name("preferences"), "protection": name("protection"),
                       "protection_level": resolve(ctx["protection"])["level"],
                       "protection_members": resolve(ctx["protection"])["members"],
                       "protection_mode": resolve(ctx["protection"])["mode"],
                       "state_digest": current["state"], "witness_length": witness_length,
                       "state_revision": int(rev["state"]), "workspace_revision": int(rev["workspace"]),
                       "protection_revision": int(rev["protection"]), "entrance": last_entrance,
                       "terminals": {command: receipt["status"] for command, receipt in terminals.items()}})
        if "lookup" in given:
            receipt = terminals.get(given["lookup"])
            result["lookup"] = receipt["status"]["kind"] if receipt else "unknown_outcome"
            result["lookup_state_revision"] = int(receipt["status"]["record"]["revision"]["state"]) if receipt and "record" in receipt["status"] else None
            result["lookup_error"] = receipt["status"].get("error", {}).get("code") if receipt else None
        return {key: result[key] for key in given["observe"]}
    return result


def dispatch(given):
    matches = [row["meaning"] for row in given["rules"] if row["selector"] == given["source"]]
    if not matches:
        return "recorded/unmapped"
    return matches[0] if len(set(matches)) == 1 else "recorded/ambiguous"


def graph_admission(nodes):
    ids = [node["id"] for node in nodes]
    require(len(ids) == len(set(ids)), FORMAT)
    for node in nodes:
        children = node.get("children", [])
        require(node["kind"] != "and" or bool(children), FORMAT)
        require(all(child in ids for child in children), FORMAT)


def blob(data):
    return {"stored": digest_ref("bytes", data), "decoded": digest_ref("bytes", data),
            "stored_bytes": str(len(data)), "decoded_bytes": str(len(data)), "encoding": "raw"}


def finite_search_structure(leaf, source):
    """I2 and I7 in the single-leaf fixture, even when EVD.41 withholds support."""
    pair = source["statements"]
    scope = source.get("negation_scope", source["scope"])
    # I2: mutual negation and the leaf's statement/scope identity bindings.
    if not (pair["E"]["negation"] == pair["N"]["id"] and pair["N"]["negation"] == pair["E"]["id"] and
            leaf["key"]["statement"] == pair["N"]["id"] and leaf["key"]["scope"] == scope["id"] and
            leaf["identity"] == scope["identity"]):
        return False
    covered, basis, form = leaf["covered"], leaf["grade"]["basis"], pair["N"].get("form")
    # I7 (b) and (c) do not depend on whether the leaf is live.
    if covered is not None and basis in {"exhaustive_computation", "exact_computation"} and \
            covered["covers"]["kind"] not in {"one_instance", "search_set"}:
        return False
    for statement, statement_scope in ((pair["E"], source["scope"]), (pair["N"], scope)):
        if statement.get("form") == {"kind": "finite"} and statement_scope["covers"]["kind"] != "one_instance":
            return False
    # I7 (a): only the two named absences defer to EVD.41's inert branch.
    if covered is not None and basis in {"certificate", "exhaustive_computation", "exact_computation"} and \
            form in ({"kind": "universal"}, {"kind": "mixed", "outer": "forall"}) and \
            covered != {key: scope[key] for key in ("covers", "input")}:
        return False
    return True


def exhaustive_refutation(leaf, source):
    """EVD.42's finite-set guard for a plain existence statement; shared by word(N) and word(E).

    I2/I7 structure, I5 and EVD.61 agreement are checked before inertness. The remaining
    conditions are I6: E's finite set, coverage, identity/contract, job and exact decisions.
    """
    search, job = leaf["search"], leaf["producing_job"]
    scope = source.get("scope")
    negation_scope = source.get("negation_scope", scope)
    if not (leaf["grade"]["basis"] == "exhaustive_computation" and scope and negation_scope and
            scope["covers"]["kind"] == "search_set" and leaf["covered"] == {key: scope[key] for key in ("covers", "input")} and
            scope["identity"] == negation_scope["identity"] and
            negation_scope["contract"] == scope["contract"] and
            search and job and search["complete"] == "yes" and
            search["stop"] == {"kind": "closed"} and job["outcome"] == "completed" and
            source["leaf"]["producer"] == job["id"]):
        return False
    pair = source["statements"]
    if search["target"] != pair["E"]["id"]:
        return False
    # The negation of an existence statement is universal, stated or not (EVD.41, Theorem D (b2)).
    if (pair["E"].get("form"), pair["N"].get("form", {"kind": "universal"})) != ({"kind": "existence"}, {"kind": "universal"}):
        return False
    declared = {key: value for key, value in scope["covers"].items() if key != "kind"}
    if search["declared"] != declared:
        return False
    # A tiny, explicitly bounded fixture language, not a general search-set interpreter.
    alphabet, length = declared["alphabet"], int_value(declared["bounds"]["length"])
    require(isinstance(alphabet, list) and all(isinstance(a, str) and len(a) == 1 for a in alphabet)
            and len(alphabet) <= 3 and 0 <= length <= 3, FORMAT)
    elements = {"".join(word) for n in range(length + 1) for word in itertools.product(alphabet, repeat=n)}
    visits = source["coverage"]["visits"]
    # complete = yes means E's own predicate decided every member; pruning needs its own lemma (EVD.5).
    return (source["coverage"]["input"] == scope["input"] and len(visits) == len(elements) and
            {visit["word"] for visit in visits} == elements and
            all(visit["qualifies"] is False and "pruned_by" not in visit for visit in visits))


def evidence_result(given):
    """Only the single-leaf H15.12 cases, under an explicit synthetic trial U0."""
    require(given["threshold"] == "trial_U0", FORMAT)
    grade = given.get("grade", {"basis": "exact_computation", "check": "producer"})
    require(grade["basis"] in {"theorem", "certificate", "exhaustive_computation", "exact_computation", "sample"}, FORMAT)
    require(grade["check"] in {"none", "producer", "independent_checker", "model_review", "human_review", "formal_proof"}, FORMAT)
    own = given.get("own_job")
    source = {"leaf": {"producer": "own" if own is not None else None}, "jobs": {}}
    if own is not None:
        source["jobs"]["own"] = {"id": "own", "outcome": own}
    if "other_job" in given:
        source["jobs"]["other"] = {"id": "other", "outcome": given["other_job"]}
    if "finite_search" in given:
        source.update(copy.deepcopy(given["finite_search"]))
    raw = canonical(source)
    job = None if own is None else {"source": digest_ref("bytes", raw), "pointer": "/jobs/own", "id": "own", "outcome": own}
    leaf = {"id": "L", "kind": "leaf", "grade": grade, "record": blob(raw), "search": given.get("search"),
            "producing_job": job, "checks": [], "membership": [], "reachability": None,
            "key": {"statement": {"kind": "named", "stage": "fixture", "commit": "fixture", "name": "S"}, "scope": "finite"},
            "identity": {"puzzle": "full", "model_id": "fixture", "menu_id": None, "menu_order": None, "contract_rev": None},
            "covered": {"covers": {"kind": "one_instance"}, "input": digest_ref("labels", b"fixture")}}
    if given.get("scope") == "search_set":
        leaf["covered"]["covers"] = {"kind": "search_set", "family": "fixture", "alphabet": "fixture", "bounds": {"length": "3"}}
    if "finite_search" in given:
        leaf["covered"] = given.get("covered", {key: source["scope"][key] for key in ("covers", "input")})
        leaf["key"]["statement"] = source["statements"]["N"]["id"]
    decoded = read_canonical(canonical(leaf))
    equal(decoded, leaf, "evidence facts round trip")
    job = decoded["producing_job"]
    if job:
        equal(job["source"], digest_ref("bytes", raw), "job source binding")
        selected = read_canonical(raw)
        for token in job["pointer"].split("/")[1:]:
            selected = selected[token]
        equal(selected, {"id": job["id"], "outcome": job["outcome"]}, "source-bound job outcome")
        equal(source["leaf"]["producer"], job["id"], "own leaf attribution")
    search = decoded["search"]
    shape, form = given["shape"], given.get("form", "finite")
    incomplete = {"error", "cancelled", "cut_off", "not_run"}
    proof = grade["basis"] == "theorem" or grade["basis"] == "certificate" and given.get("complete_certificate", False)
    holder = next((statement for statement in read_canonical(raw).get("statements", {}).values()
                   if statement["id"] == decoded["key"]["statement"]), {})
    holder_form = holder.get("form")
    reads_coverage = holder_form is None or holder_form in ({"kind": "universal"}, {"kind": "mixed", "outer": "forall"})
    absent_coverage = (decoded["covered"] is None and reads_coverage and
                       grade["basis"] in {"certificate", "exhaustive_computation", "exact_computation"})
    # EVD.41: with no `form` fact, a leaf that I7 (a) would flag is inert instead of row 0.
    holder_scope = source.get("negation_scope", source.get("scope"))
    flagged_without_form = bool(holder and holder_form is None and holder_scope and decoded["covered"] is not None and
                                grade["basis"] in {"certificate", "exhaustive_computation", "exact_computation"} and
                                decoded["covered"] != {key: holder_scope[key] for key in ("covers", "input")})
    inert = own == "not_run" or bool(search and search["job"] == "not_stated" and
                                     (shape == "negative" or grade["basis"] == "exhaustive_computation")) or \
        absent_coverage or flagged_without_form
    code, row, word = None, 7, "open"
    # EVD.61 agreement, I5, row-0 structure, EVD.41, then I6 on live leaves only.
    if search and job and search["job"] != job["outcome"]:
        code = "EVD:evidence.inconsistent_fields"
    elif shape == "negative" and own in incomplete:
        code = "EVD:evidence.negative_without_completion"
    elif shape != "empty" and (shape == "negative" or grade["basis"] == "exhaustive_computation") and search and (
            search["complete"] == "no" or search["job"] in incomplete):
        code = "EVD:evidence.negative_without_completion"
    elif "finite_search" in given and not finite_search_structure(decoded, source):
        code = "EVD:evidence.inconsistent_fields"
    elif inert:
        pass
    elif shape == "negative" and form == "existence" and not (proof or exhaustive_refutation(decoded, read_canonical(raw))):
        code = "EVD:evidence.inconsistent_fields"
    elif shape == "negative" and form == "mixed_exists" and not proof:
        code = "EVD:evidence.inconsistent_fields"
    elif shape == "negative" and (
            (form in {"universal", "mixed_forall"} and not given.get("membership", False)) or
            (form == "mixed_forall" and not given.get("complete_negative_at_witness", False))):
        code = "EVD:evidence.inconsistent_fields"
    stands = shape != "empty" and not inert and grade["basis"] != "sample" and grade["check"] != "none"
    if code:
        row, word = 0, "typed inconsistency"
    elif stands:
        row, word = (1, "refuted") if shape == "negative" and given.get("query", "E") == "E" else (2, "finite instance")
    elif search and search["complete"] == "no" and search["job"] != "not_run" and search["stop"]["kind"] in {"deterministic_budget", "declared_limit"}:
        row, word = 5, "not found within budget"
    elif own in {"error", "cancelled", "cut_off"} or search and search["job"] in {"error", "cancelled", "cut_off"}:
        row, word = 6, "incomplete"
    return {"row": row, "word": word, "code": code, "producing_job": own,
            "grade": decoded["grade"], "search": decoded["search"]}


def word_admission(record, blobs, records, inverse, menu, visiting=()):
    supported(record)
    digest = sha(canonical(record))
    require(digest not in visiting, FORMAT)
    require(all(record["identity"][pin] == menu[pin] for pin in ("menu_id", "menu_order")),
            "KN1:input.identity_mismatch")
    payload = record["payload"]
    require(set(payload) == {"alphabet", "start", "length", "letters", "chunks", "parts", "annotations"}, FORMAT)
    require(payload["alphabet"] == "menu", FORMAT)
    chunks = []
    for ref in payload["chunks"]:
        raw = blobs.get(ref["stored"]["sha256"])
        require(raw is not None, FORMAT)
        raw = bytes.fromhex(raw)
        require(blob(raw) == ref, DIGEST)
        chunks.append(raw)
    letters = read_chunks(chunks, int_value(payload["length"]), payload["letters"]["sha256"],
                          record["identity"]["menu_order"], menu["menu_order"])
    parts = {part["id"]: part["word"] for part in payload["parts"]}
    require(list(parts) == sorted(parts) and len(parts) == len(payload["parts"]), FORMAT)

    def external(ref):
        supported(ref)
        target = records.get(ref["content"]["sha256"])
        require(target is not None, FORMAT)
        supported(target)
        require(target["format"] == ref["format"] and target["version"] == ref["version"], FORMAT)
        require(digest_ref("content", canonical(target)) == ref["content"], DIGEST)
        require(target["identity"] == record["identity"], "KN1:input.identity_mismatch")
        return word_admission(target, blobs, records, inverse, menu, visiting + (digest,))

    for ref in parts.values():
        external(ref)  # Included definitions need closure even if not used by an annotation.
    local = {}
    for annotation in payload["annotations"]:
        require(set(annotation) == {"name", "start", "end", "relation"}, FORMAT)
        start, end = int_value(annotation["start"]), int_value(annotation["end"])
        require(0 <= start <= end <= len(letters) and annotation["name"] not in local, FORMAT)
        span, relation = letters[start:end], annotation["relation"]
        if relation:
            require(set(relation) == {"kind", "word"} and relation["kind"] in {"same_as", "inverse_of"}, FORMAT)
            target = relation["word"]
            require(isinstance(target, dict) and len(target) == 1, FORMAT)
            tag, value = next(iter(target.items()))
            require(tag in {"local", "part", "ref"}, FORMAT)
            if tag == "local":
                require(value in local, FORMAT)
                expected = local[value]
            else:
                require(tag != "part" or value in parts, FORMAT)
                expected = external(parts[value] if tag == "part" else value)
            if relation["kind"] == "inverse_of":
                require(all(str(item) in inverse for _, item in expected), FORMAT)
                expected = [(grip, inverse[str(item)]) for grip, item in reversed(expected)]
            require(span == expected, CANON)
        local[annotation["name"]] = span
    return letters


def relation_result(given, examples):
    fixture = copy.deepcopy(examples["ji23"])
    record = fixture["record"]
    payload = record["payload"]
    case = given["case"]
    if case in {"part", "ref"}:
        ref = fixture["part_ref"]
        if case == "part":
            payload["parts"] = [{"id": "A-part", "word": ref}]
        for annotation in payload["annotations"][1:]:
            annotation["relation"]["word"] = {"part": "A-part"} if case == "part" else {"ref": ref}
    elif case == "missing":
        payload["annotations"][1]["relation"]["word"] = {"local": "missing"}
    elif case == "cycle":
        payload["annotations"][0]["relation"] = {"kind": "same_as", "word": {"local": "B"}}
    elif case == "changed_inverse":
        raw = bytes.fromhex("0c000c000c00")
        payload["chunks"] = [blob(raw)]
        payload["letters"] = digest_ref("bytes", raw)
        fixture["blobs"][sha(raw)] = raw.hex()
    elif case == "missing_chunks":
        del payload["chunks"]
    elif case == "missing_part":
        payload["parts"] = [{"id": "A-part", "word": fixture["part_ref"]}]
        fixture["records"] = {}
    elif case == "part_future_version":
        # FP.10: a part whose Ref and record both declare an unknown version is refused before it is read.
        target = copy.deepcopy(fixture["records"][fixture["part_ref"]["content"]["sha256"]])
        target["version"] = 2
        ref = {**fixture["part_ref"], "version": 2, "content": digest_ref("content", canonical(target))}
        fixture["records"] = {ref["content"]["sha256"]: target}
        payload["parts"] = [{"id": "A-part", "word": ref}]
    elif case in {"menu_id", "menu_order"}:
        record["identity"][case] = "0" * 64
    return outcome(lambda: word_admission(record, fixture["blobs"], fixture["records"], fixture["inverse"], fixture["menu"]))


def alias_result(given):
    table = json.loads((ROOT / "docs/progress/1.0/command-aliases.json").read_bytes())
    pin = given["pin"]
    require(pin["version"] == table["version"] and pin["profile"] == table["canonical_form"], DIGEST)
    body = {key: value for key, value in table.items() if key != "content_digest"}
    raw = json.dumps(body, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    if given.get("append_lf"):
        raw += b"\n"
    require(sha(raw) == table["content_digest"] == pin["content_digest"], DIGEST)
    matches = [case["target"] for row in table["rows"] for case in row["cases"] if case["legacy_id"] == given["legacy_id"]]
    require(len(matches) == 1, FORMAT)
    return matches[0]


def pin_result(relative, recorded, actual, frozen):
    if recorded == actual:
        return "matched"
    chapter = relative.startswith("docs/architecture/1.0/language/architecture/") and relative.endswith(".md")
    if chapter and not frozen:
        return "chapter_moved"
    raise Refusal(DIGEST)


def check_inventory():
    inventory = json.loads((HERE / "candidate.json").read_bytes())
    require(type(inventory.get("frozen")) is bool, FORMAT)
    equal(inventory["candidate_sha256"], sha(canonical(inventory["candidate"])), "candidate identity")
    for entry in inventory["candidate"] + inventory["inputs"]:
        relative = entry["path"]
        path = ROOT / relative
        require(not Path(relative).is_absolute() and ".." not in Path(relative).parts, FORMAT)
        actual = sha(path.read_bytes())
        status = pin_result(relative, entry["sha256"], actual, inventory["frozen"])
        if status == "chapter_moved":
            print(f"draft chapter moved: {relative}: {entry['sha256']} -> {actual}")


def playback_result(given):
    """PS1.44 and EV4.18: map a recorded undo or redo target to the played action before anything is undone."""
    recorded = [entry["recorded"] for entry in given["actions"]]
    require(recorded == sorted(set(recorded)), CANON)
    stop = given.get("stopped")
    if stop is not None:
        require(len(stop) == 1 and (isinstance(stop.get("code"), str) or
                                    stop.get("outcome") in {"refused", "failed", "cancelled", "unknown_outcome"}), FORMAT)
        return stop
    require(given["direction"] in {"undo", "redo"}, FORMAT)
    played = {entry["recorded"]: entry["played"] for entry in given["actions"]}
    target, hint = given["recorded_target"], given["hint_target"]
    if target not in played or hint is None or played[target] != hint:
        return {"stop": "EV4:stale.recording_step", "undone": False}
    return {"stop": None, "undone": True}


def playback_from_result(given):
    """EV4.22: what recording.play does from the state saved for one recording, after pending reconciliation."""
    state, start, steps = given["state"], given.get("from"), given["steps"]
    require(start is None or 0 <= start < steps, STATE)
    if state is None:
        return {"playback": "new", "step": start or 0, "map": "empty"}
    status = state["status"]
    require(status != "running" and state.get("child") not in {"accepted", "running"}, STATE)
    if status in {"waiting", "stopped"} and state.get("outcome") != "unknown_outcome":
        if start is None or start >= state["step"]:
            return {"playback": "same", "step": state["step"] if start is None else start, "map": "kept"}
        return {"playback": "new", "step": start, "map": "empty"}
    require(start is not None, STATE)
    return {"playback": "new", "step": start, "map": "empty"}


def playback_trace_result(given):
    """EV4.22 with EV4.18: a rewind opens a new playback, so a later undo reaches only what that playback played."""
    actions = []
    for event in given["events"]:
        if "step" in event:
            actions = sorted([entry for entry in actions if entry["recorded"] != event["step"]["recorded"]] + [event["step"]],
                             key=lambda entry: entry["recorded"])
        elif playback_from_result(event["rewind"])["map"] == "empty":
            actions = []
    return playback_result({"actions": actions, "direction": given["direction"],
                            "recorded_target": given["recorded_target"], "hint_target": given["hint_target"]})


def playback_argument_result(given):
    """EV4.14: a step may name the current revision or a fresh filter preview; anything else stops as stale."""
    named = given["named"]
    require(named["kind"] in {"revision", "filter"}, FORMAT)
    current = named["value"] == given["current"] if named["kind"] == "revision" else given["fresh"] is True
    return {"stop": None, "played": True} if current else {"stop": "EV4:stale.recording_step", "played": False}


def playback_storage_result(given):
    """FP.37: a workspace wrapper names a complete playback record; every field is read and `last` resolves."""
    wrapper = read_canonical(canonical(given["wrapper"]))
    record_header(wrapper)
    provenance(wrapper["provenance"])
    payload = wrapper["payload"]
    require(set(payload) == {"id", "kind", "value", "dependencies", "action", "revision"}, FORMAT)
    value = payload["value"]
    supported(value)
    require(payload["kind"] == value["format"] == ident("playback"), FORMAT)
    records = {sha(canonical(record)): record for record in given["records"]}
    record = records.get(value["content"]["sha256"])
    require(record is not None, DIGEST)
    record_header(record)
    require(record["format"] == value["format"] and record["version"] == value["version"], FORMAT)
    require(record["provenance"] is None, FORMAT)
    state = record["payload"]
    require(set(state) == {"playback", "recording", "step", "pending", "actions", "status", "last"}, FORMAT)
    require(isinstance(state["playback"], str) and state["recording"].get("role") == "content", FORMAT)
    require(int_value(state["step"]) >= 0, CANON)
    pending = state["pending"]
    require(pending is None or (set(pending) == {"command", "request"} and isinstance(pending["command"], str)
                                and pending["request"].get("role") == "content"), FORMAT)
    numbers = []
    for entry in state["actions"]:
        require(set(entry) == {"recorded", "played"}, FORMAT)
        require(int_value(entry["played"]) >= 0, CANON)
        numbers.append(int_value(entry["recorded"]))
    require(all(low < high for low, high in zip(numbers, numbers[1:])), CANON)
    status = state["status"]
    if status.get("kind") == "waiting":
        require(set(status) == {"kind", "step"} and int_value(status["step"]) >= 0, FORMAT)
    elif status.get("kind") == "stopped":
        stop = status.get("stop", {})
        fixed = (stop.get("kind") == "code" and set(stop) == {"kind", "code"} and isinstance(stop["code"], str)) or (
            stop.get("kind") == "outcome" and set(stop) == {"kind", "outcome"}
            and stop["outcome"] in {"refused", "failed", "cancelled", "unknown_outcome"})
        require(set(status) == {"kind", "stop"} and fixed, FORMAT)
    else:
        require(status in ({"kind": "running"}, {"kind": "finished"}), FORMAT)
    last, command = state["last"], None
    if last is not None:
        supported(last)
        require(last["format"] == ident("receipt"), FORMAT)
        receipt = records.get(last["content"]["sha256"])
        require(receipt is not None, DIGEST)
        record_header(receipt)
        require(receipt["format"] == last["format"] and receipt["version"] == last["version"], FORMAT)
        require(set(receipt["payload"]) == {"command", "request", "job", "status"}, FORMAT)
        command = receipt["payload"]["command"]
    return {"sha256": sha(canonical(wrapper)), "step": state["step"], "status": status["kind"],
            "actions": len(numbers), "last": command}


def pick_result(given):
    """FP.17, FP.38: a non-null pick resolves to its record under each policy form; a missing record fails closed."""
    block = read_canonical(canonical(given["block"]))
    provenance(block)
    records = {sha(canonical(record)): record for record in given["records"]}
    pick = block["pick"]
    require(pick is not None, PROVENANCE)
    reference = pick["record"]
    supported(reference)
    require(reference["format"] == ident("pick"), PROVENANCE)
    record = records.get(reference["content"]["sha256"])
    require(record is not None, PROVENANCE)
    record_header(record)
    require(record["format"] == reference["format"] and record["version"] == reference["version"], PROVENANCE)
    require(record["provenance"] is None, FORMAT)
    payload = record["payload"]
    require(set(payload) == {"list", "fixed", "options", "chosen", "target"} and type(payload["fixed"]) is bool, FORMAT)
    require(payload["chosen"] == pick["chosen"], PROVENANCE)
    return {"chosen": pick["chosen"]["sha256"], "block": sha(canonical(block))}


def editions_result(given):
    """FP.19, PS2.6: with no cache, a display-function change keeps the old pin and records the new edition."""
    fields = {"order", "summary", "display", "invalid_pairs", "threshold", "import"}
    old, new = given["old"], given["new"]
    require(set(old) == fields and set(new) == fields, FORMAT)
    require(old["import"] is not None and new["import"] is not None and given["caches"] == [], FORMAT)
    return {"changed": sorted(field for field in fields if old[field] != new[field]), "facts": sha(canonical(given["facts"])),
            "old": sha(canonical(old)), "new": sha(canonical(new))}


def lean_args(name, args, table, bound):
    row = table["symbols"].get(name)
    require(row is not None and row["arity"] == len(args), UNTRANSLATABLE)
    text = [row["lean"], *(lean_term(arg, table, bound) for arg in args)]
    return text[0] if not args else "(" + " ".join(text) + ")"


def lean_term(term, table, bound):
    kind = term.get("kind")
    if kind == "var":
        require(term["name"] in bound, UNTRANSLATABLE)
        return term["name"]
    if kind == "nat":
        require(int_value(term["value"]) >= 0, UNTRANSLATABLE)
        return term["value"]
    if kind == "int":
        return "(" + str(int_value(term["value"])) + " : Int)"
    if kind == "rat":
        require(int_value(term["d"]) > 0, UNTRANSLATABLE)
        return "((" + str(int_value(term["n"])) + " : Rat) / " + term["d"] + ")"
    require(kind == "sym", UNTRANSLATABLE)
    return lean_args(term["name"], term["args"], table, bound)


def lean_sort(sort, table):
    if sort.get("kind") in {"Nat", "Int", "Rat"}:
        return sort["kind"]
    require(sort.get("kind") == "named" and sort.get("name") in table["sorts"], UNTRANSLATABLE)
    return table["sorts"][sort["name"]]


def lean_prop(prop, table, bound):
    kind = prop.get("kind")
    if kind in {"true", "false"}:
        return kind.capitalize()
    if kind in {"eq", "le", "lt"}:
        mark = {"eq": "=", "le": "≤", "lt": "<"}[kind]
        return f"({lean_term(prop['a'], table, bound)} {mark} {lean_term(prop['b'], table, bound)})"
    if kind == "not":
        return f"(¬ {lean_prop(prop['p'], table, bound)})"
    if kind in {"and", "or", "implies"}:
        mark = {"and": "∧", "or": "∨", "implies": "→"}[kind]
        return f"({lean_prop(prop['p'], table, bound)} {mark} {lean_prop(prop['q'], table, bound)})"
    if kind in {"forall", "exists"}:
        mark = {"forall": "∀", "exists": "∃"}[kind]
        inner = lean_prop(prop["p"], table, bound | {prop["name"]})
        return f"({mark} {prop['name']} : {lean_sort(prop['sort'], table)}, {inner})"
    require(kind == "pred", UNTRANSLATABLE)
    return lean_args(prop["name"], prop["args"], table, bound)


def obligation_result(given):
    """PS1.45, PS1.46: a typed statement exports as Lean 4 source with no proof; anything else writes nothing."""
    identity, table = given["statement"]["id"], given["table"]
    require(identity.get("kind") == "typed", UNTRANSLATABLE)
    require(table["vocabulary"] == "c600-obligation-vocabulary-draft-1", UNTRANSLATABLE)
    body = read_canonical(canonical(given["body"]))
    require(identity["content"] == digest_ref("content", canonical(body)), DIGEST)
    require(set(body) == {"hypotheses", "conclusion"}, UNTRANSLATABLE)
    empty = frozenset()
    head = ["theorem obligation"] + [f"(h{index} : {lean_prop(prop, table, empty)})"
                                     for index, prop in enumerate(body["hypotheses"])]
    source = " ".join(head) + f" : {lean_prop(body['conclusion'], table, empty)} := by sorry\n"
    return {"format": "c600-proof-obligation-draft", "vocabulary": table["vocabulary"],
            "source": source, "sha256": sha(source.encode("utf-8"))}


# FP.24 Path B packages (migration-format-freeze ruling, MFF-01). `path_b_package` builds the synthetic
# golden packages in memory; `path_b_read` validates one from its archive bytes alone.
ZIP_DATE = (1 << 5) | 1  # DOS date 1980-01-01; the DOS time field is zero
NAMED_MEMBERS = {"session/record.json": "session", "session/checkpoint-paths.json": "paths",
                 "workspace/macros.json": "macros", "workspace/keymap.json": "keymap",
                 "workspace/workspace.json": "workspace", "raw/session.sqlite3": "raw"}
RECORD_ROLES = frozenset({"session", "paths", "macros", "keymap", "workspace", "record"})
WORKSPACE_KINDS = {"workspace/macros.json": "macro-set", "workspace/keymap.json": "key-map",
                   "workspace/workspace.json": "workspace"}  # FP.24: the record each workspace member holds
CARRIER_MEMBER = re.compile(r"records/([0-9a-f]{64})\.json\Z|blobs/([0-9a-f]{64})\Z")
LABEL_MEMBER = re.compile(r"session/checkpoints/(?:head|0|[1-9][0-9]*)\.labels\Z")


def member_role(name):
    """FP.24's role for a member name; None for a name outside the package layout."""
    if name in NAMED_MEMBERS:
        return NAMED_MEMBERS[name]
    carrier = CARRIER_MEMBER.fullmatch(name)
    if carrier:
        return "record" if carrier.group(1) else "blob"
    if LABEL_MEMBER.fullmatch(name):
        return "labels"
    return "log" if name.startswith("logs/") and len(name) > 5 else None


def ref_links(value, refs, blobs):
    """Collect content Refs and BlobRefs; a bare DigestRef or Pin names bytes and is not a dependency."""
    if isinstance(value, dict):
        if set(value) == {"format", "version", "content"}:
            refs.append(value)
        elif set(value) == {"stored", "decoded", "stored_bytes", "decoded_bytes", "encoding"}:
            blobs.append(value)
        else:
            for child in value.values():
                ref_links(child, refs, blobs)
    elif isinstance(value, list):
        for child in value:
            ref_links(child, refs, blobs)


def zip_store(members):
    """FP.23's framing: STORE, UTF-8 flag only, 1980-01-01 00:00:00, version 20, no extras, comments or attributes."""
    body, directory = bytearray(), bytearray()
    for name, data in members:
        encoded = name.encode("utf-8")
        common = struct.pack("<HHHHHIIIHH", 20, 0x800, 0, 0, ZIP_DATE, zlib.crc32(data), len(data), len(data),
                             len(encoded), 0)
        directory += struct.pack("<IH", 0x02014B50, 20) + common + struct.pack("<HHHII", 0, 0, 0, 0, len(body)) + encoded
        body += struct.pack("<I", 0x04034B50) + common + encoded + data
    return bytes(body + directory + struct.pack("<IHHHHIIH", 0x06054B50, 0, 0, len(members), len(members),
                                                len(directory), len(body), 0))


def zip_read(data):
    """Read only FP.23's framing; members come back in archive order."""
    charge("bundle_stored", len(data))
    require(len(data) >= 22, FORMAT)
    sig, disk, start_disk, here, total, size, offset, comment = struct.unpack_from("<IHHHHIIH", data, len(data) - 22)
    require(sig == 0x06054B50 and disk == start_disk == comment == 0 and here == total >= 1, FORMAT)
    require(offset + size == len(data) - 22, FORMAT)
    charge("bundle_members", total - 1)
    members, folded, at, position = [], set(), 0, offset
    for _ in range(total):
        require(position + 46 <= offset + size, FORMAT)
        sig, made = struct.unpack_from("<IH", data, position)
        common = data[position + 6:position + 32]
        need, flags, method, time, date, crc, csize, usize, length, extra = struct.unpack("<HHHHHIIIHH", common)
        note, disk, internal, external, local = struct.unpack_from("<HHHII", data, position + 32)
        raw_name = data[position + 46:position + 46 + length]
        position += 46 + length
        require(sig == 0x02014B50 and made == need == 20 and flags == 0x800 and method == time == 0 and
                date == ZIP_DATE and csize == usize and extra == note == disk == internal == external == 0, FORMAT)
        require(local == at and data[at:at + 30 + length] == struct.pack("<I", 0x04034B50) + common + raw_name, FORMAT)
        content = data[at + 30 + length:at + 30 + length + csize]
        at += 30 + length + csize
        require(at <= offset, FORMAT)
        require(zlib.crc32(content) == crc, DIGEST)
        try:
            name = raw_name.decode("utf-8")
        except UnicodeDecodeError as exc:
            raise Refusal(FORMAT) from exc
        require(all(part not in {"", ".", ".."} for part in name.split("/")) and "\\" not in name and ":" not in name,
                FORMAT)
        require(name.casefold() not in folded, FORMAT)  # duplicates and case aliases
        folded.add(name.casefold())
        members.append((name, content))
    require(at == offset and position == offset + size, FORMAT)
    return members


def path_b_package(fixture, raw_copy, reference, examples, mutation=None):
    """FP.24's synthetic golden package: `fresh-session` (F12) or `abandoned-branch` (F10). Not importer output."""
    identity = {"puzzle": "full", "model_id": reference.manifest["model_id"],
                "menu_id": None, "menu_order": None, "contract_rev": None}
    block = examples["records"]["manual_capability"]["provenance"]
    stored = {}

    def keep(record):
        raw = canonical(record)
        stored[sha(raw)] = raw
        return {"format": record["format"], "version": record["version"], "content": digest_ref("content", raw)}

    def wire(kind, payload):
        return keep({"format": ident(kind), "version": 1, "identity": identity, "payload": payload,
                     "provenance": None if kind in {"recipe", "checkpoint", "replay-path", "migration"} else block})

    def pin(text):
        return {"version": "fixture-1", "source": digest_ref("source", text.encode("utf-8"))}

    def preferences(show):
        entry = {"name": "show_frame", "value": show, "outcome": "converted",
                 "source": digest_ref("bytes", b"show_frame=" + (b"true" if show else b"false"))}
        return wire("preferences", {"contract": pin("preferences"), "entries": [entry], "protection": protection,
                                    "conversion": None})

    branch = {"fresh-session": False, "abandoned-branch": True}[fixture]
    protection = wire("protection-set", {"level": "labels", "members": [{"kind": "orbit", "id": 0}], "mode": "net",
                                         "contract": pin("protection"), "origin": None})
    contract = {"version": "retained-0.4", "source": digest_ref("source", (ROOT / "core.py").read_bytes())}
    words = {"1": [1, 2], "2": [3], "3": [-4]} if branch else {}
    parents = {"1": None, "2": "1", "3": "1"}
    states, recipes, events = {"": i32_bytes(range(reference.count))}, {}, []
    for event, word in words.items():
        recipes[event] = wire("recipe", {"steps": [{"kind": "word", "moves": word}], "primitive_count": int_text(len(word)),
                                         "expansion": digest_ref("recipe_expansion", i32_bytes(word)), "contract": contract})
        chain, at = [], event
        while at:
            chain.insert(0, at)
            at = parents[at]
        states[event] = i32_bytes(reference.replay([move for step in chain for move in words[step]])[0])
        events.append({"id": event, "parent": parents[event], "value": recipes[event],
                       "pre": digest_ref("labels", states[parents[event] or ""]),
                       "post": digest_ref("labels", states[event]), "preferences": None})
    head, offered = ("3", ["2"]) if branch else ("", [])
    paths = {head: wire("replay-path", {"root": None, "head": head, "events": events})}
    prefs = {head: preferences(True)}
    for event in offered:
        paths[event] = wire("replay-path", {"root": None, "head": event, "events": events})
        prefs[event] = preferences(False)
    labels = {point: states[point] for point in [head] + offered}
    if mutation and mutation["kind"] == "exchange_labels":
        labels[head], labels[offered[0]] = labels[offered[0]], labels[head]
    checkpoints = {}
    for point in [head] + offered:
        checkpoints[point] = wire("checkpoint", {
            "id": "current" if point == head else "branch-" + point,
            "name": "Current state" if point == head else "Abandoned branch",
            "mode": "jumble_off", "state": {"digest": digest_ref("labels", labels[point]), "data": blob(labels[point])},
            "root": {"kind": "solved"}, "path": paths[point], "preferences": prefs[point],
            "episode": None, "exit": None, "assistance": block})
    session = keep({"format": ident("session-record"), "version": 1, "head": checkpoints[head],
                    "checkpoints": [checkpoints[point] for point in offered], "preferences": prefs[head],
                    "attempt": None, "timer": None, "unmapped": []})
    conversion = wire("migration", {
        "source": {"format": "magic600-0.4-session", "version": 1, "edition": None,
                   "bytes": digest_ref("bytes", b"synthetic locked session copy")},
        "target": session, "converter": pin("golden Path B converter"), "inputs": [],
        "outcomes": [{"item": "session", "status": "converted", "reason": "synthetic golden package"}],
        "checks": [], "semantic_changes": []})
    named = {"session/record.json": session, "session/checkpoint-paths.json": paths[head]}
    carried = {}
    if branch:
        macro = wire("macro", {"id": "m1", "name": "First event again", "revision": "0", "value": recipes["1"],
                               "metadata": {}, "evidence": []})
        named["workspace/macros.json"] = wire("macro-set", {"entries": [macro], "origin": None})
        source = b"highlight orbit 0\n"
        packed = pack_gzip(source)
        carried[sha(packed)] = packed
        source_ref = {"stored": digest_ref("bytes", packed), "decoded": digest_ref("bytes", source),
                      "stored_bytes": str(len(packed)), "decoded_bytes": str(len(source)),
                      "encoding": "c600-gzip-stored-1"}
        rule = wire("filter", {"contract": pin("filter"), "rules": [{"expr": "orbit 0", "style": "highlight"}],
                               "named_sets": [], "source": source_ref, "conversion": None})
        named["workspace/workspace.json"] = wire("workspace", {"id": "filter", "kind": ident("filter"), "value": rule,
                                                               "dependencies": [], "action": "converted",
                                                               "revision": "0"})
    members = {name: stored[ref["content"]["sha256"]] for name, ref in named.items()}
    held = {ref["content"]["sha256"] for ref in named.values()}
    members.update((f"records/{key}.json", raw) for key, raw in stored.items() if key not in held)
    members.update((f"blobs/{key}", raw) for key, raw in carried.items())
    members["session/checkpoints/head.labels"] = labels[head]
    members.update((f"session/checkpoints/{index}.labels", labels[point]) for index, point in enumerate(offered))
    if raw_copy:
        members["raw/session.sqlite3"] = b"synthetic raw copy; never a resolution input\n"
    if mutation and mutation["kind"] == "drop":
        del members[mutation["path"]]
    elif mutation and mutation["kind"] == "add":
        raw = canonical(mutation["record"])
        members[f"records/{sha(raw)}.json"] = raw
    elif mutation and mutation["kind"] == "duplicate":
        members[f"records/{sha(members[mutation['path']])}.json"] = members[mutation["path"]]
    elif mutation and mutation["kind"] == "rename":
        members[mutation["to"]] = members.pop(mutation["path"])
    else:
        require(not mutation or mutation["kind"] == "exchange_labels", FORMAT)
    entries = [{"path": name, "bytes": str(len(members[name])), "sha256": sha(members[name]), "role": member_role(name)}
               for name in sorted(members, key=lambda name: name.encode("utf-8"))]
    manifest = {"format": "C600-MIGRATE", "format_version": 1,
                "producer": {"name": "golden-path-b", "version": "1",
                             "source": digest_ref("source", b"golden Path B synthetic producer")},
                "model_id": reference.manifest["model_id"], "created_utc": "1980-01-01T00:00:00Z",
                "head_state_hash": sha(labels[head]), "head_event_id": head, "entries": entries,
                "options": {"include_logs": False, "raw_copy": raw_copy}, "privacy": {"publication": "private"},
                "limits": "c600-limits-1", "conversion": conversion}
    return zip_store([("manifest.json", canonical(manifest))] + [(entry["path"], members[entry["path"]])
                                                                 for entry in entries])


def path_b_read(data, reference):
    """FP.24 from the archive bytes alone: entries, carriers, closure, then each offered path's replay."""
    members = zip_read(data)
    require(members[0][0] == "manifest.json", FORMAT)
    manifest = read_canonical(members[0][1])
    require(isinstance(manifest, dict) and {"format", "format_version"} <= manifest.keys(), FORMAT)
    require(manifest["format"] == "C600-MIGRATE" and type(manifest["format_version"]) is int and
            manifest["format_version"] == 1, VERSION)
    require(set(manifest) == {"format", "format_version", "producer", "model_id", "created_utc", "head_state_hash",
                              "head_event_id", "entries", "options", "privacy", "limits", "conversion"}, FORMAT)
    require(manifest["limits"] == "c600-limits-1", VERSION)
    require(manifest["model_id"] == reference.manifest["model_id"] and
            manifest["privacy"] == {"publication": "private"}, FORMAT)
    options, entries = manifest["options"], manifest["entries"]
    require(isinstance(options, dict) and set(options) == {"include_logs", "raw_copy"} and
            all(type(flag) is bool for flag in options.values()), FORMAT)
    require(isinstance(entries, list) and all(isinstance(entry, dict) and set(entry) == {"path", "bytes", "sha256", "role"}
                                              and isinstance(entry["path"], str) for entry in entries), FORMAT)
    names = [entry["path"] for entry in entries]
    require(names == sorted(set(names), key=lambda name: name.encode("utf-8")), FORMAT)
    require(names == [name for name, _ in members[1:]], FORMAT)
    # Entries are checked before any logical content is read.
    records, blobs, labels, digest_of = {}, {}, {}, {}
    for entry, (name, raw) in zip(entries, members[1:]):
        role = member_role(name)
        require(role is not None and entry["role"] == role, FORMAT)
        require(entry["bytes"] == str(len(raw)) and entry["sha256"] == sha(raw), DIGEST)
        carrier = CARRIER_MEMBER.fullmatch(name)
        require(carrier is None or entry["sha256"] in carrier.groups(), DIGEST)
        digest_of[name] = entry["sha256"]
        if role in RECORD_ROLES or role == "blob":
            index = records if role in RECORD_ROLES else blobs
            require(entry["sha256"] not in index, FORMAT)  # a second carrier of one digest
            index[entry["sha256"]] = (name, raw)
        elif role == "labels":
            labels[name] = raw
    require(options["raw_copy"] == ("raw/session.sqlite3" in digest_of), FORMAT)
    require(options["include_logs"] == any(entry["role"] == "log" for entry in entries), FORMAT)
    # The closure: the conversion Ref and every named record member, followed through each resolved record.
    seen, used, label_refs, blob_refs = {}, set(), {}, []
    pending = [(manifest["conversion"], None)] + [(None, digest_of[name]) for name, role in NAMED_MEMBERS.items()
                                                  if role in RECORD_ROLES and name in digest_of]
    while pending:
        ref, key = pending.pop()
        if ref is not None:
            supported(ref)  # FP.10: the Ref's own pair before it is followed
            require(isinstance(ref["content"], dict) and set(ref["content"]) == {"role", "sha256"} and
                    ref["content"]["role"] == "content", FORMAT)
            key = ref["content"]["sha256"]
        require(key in records, FORMAT)
        if key not in seen:
            name, raw = records[key]
            record = read_canonical(raw)
            supported(record)
            if record["format"] != ident("session-record"):
                record_header(record)
                require(record["provenance"] is not None or
                        record["format"] not in {ident("word"), ident("macro"), ident("workspace")}, FORMAT)
            seen[key], walk = record, copy.deepcopy(record)
            used.add(name)
            if record["format"] == ident("checkpoint"):
                state = walk["payload"].get("state") if isinstance(walk["payload"], dict) else None
                require(isinstance(state, dict) and "data" in state, FORMAT)
                label_refs[key] = state.pop("data")  # carried by its label member, not blobs/
            refs = []
            ref_links(walk, refs, blob_refs)
            pending.extend((child, None) for child in refs)
        if ref is not None:
            require(seen[key]["format"] == ref["format"] and seen[key]["version"] == ref["version"], FORMAT)
    for name, kind in WORKSPACE_KINDS.items():  # retained exchange bytes are outside the golden family (FP.26)
        require(name not in digest_of or seen[digest_of[name]]["format"] == ident(kind), FORMAT)

    def target(ref):
        require(isinstance(ref, dict) and isinstance(ref.get("content"), dict) and
                ref["content"].get("sha256") in seen, FORMAT)
        return seen[ref["content"]["sha256"]]

    decoded = sum(len(raw) for _, raw in members)
    for blob_ref in blob_refs:
        stored_ref, decoded_ref = blob_ref["stored"], blob_ref["decoded"]
        require(isinstance(stored_ref, dict) and stored_ref.get("role") == "bytes" and
                stored_ref.get("sha256") in blobs, FORMAT)
        name, raw = blobs[stored_ref["sha256"]]
        require(blob_ref["stored_bytes"] == str(len(raw)), DIGEST)
        require(blob_ref["encoding"] in {"raw", "c600-gzip-stored-1"}, FORMAT)
        content = raw if blob_ref["encoding"] == "raw" else unpack_gzip(raw)
        require(blob_ref["decoded_bytes"] == str(len(content)) and decoded_ref == digest_ref("bytes", content), DIGEST)
        if name not in used:
            decoded += len(content) - len(raw)
        used.add(name)
    charge("bundle_decoded", decoded)
    require("session/record.json" in digest_of and "session/checkpoint-paths.json" in digest_of, FORMAT)
    session = seen[digest_of["session/record.json"]]
    require(session["format"] == ident("session-record") and isinstance(session.get("checkpoints"), list) and
            set(session) == {"format", "version", "head", "checkpoints", "preferences", "attempt", "timer", "unmapped"},
            FORMAT)
    offered = [("head", session["head"])] + [(str(index), ref) for index, ref in enumerate(session["checkpoints"])]
    keys = []
    for _, ref in offered:
        target(ref)
        keys.append(ref["content"]["sha256"])
    require(len(set(keys)) == len(keys) and set(keys) == set(label_refs), FORMAT)
    for (tag, _), key in zip(offered, keys):
        member = f"session/checkpoints/{tag}.labels"
        require(member in labels, FORMAT)
        require(label_refs[key] == blob(labels[member]), DIGEST)
        used.add(member)
    require(set(labels) <= used and {name for name, _ in [*records.values(), *blobs.values()]} <= used, FORMAT)
    main_path = seen[digest_of["session/checkpoint-paths.json"]]
    require(main_path["format"] == ident("replay-path") and isinstance(main_path["payload"], dict) and
            set(main_path["payload"]) == {"root", "head", "events"}, FORMAT)
    root, events = main_path["payload"]["root"], main_path["payload"]["events"]
    require(root is None and isinstance(events, list), FORMAT)  # the golden packages start from the solved root
    require(main_path["payload"]["head"] == manifest["head_event_id"], REPLAY)
    charge("archive_events", len(events))
    by_id = {}
    for event in events:
        require(isinstance(event, dict) and set(event) == {"id", "parent", "value", "pre", "post", "preferences"}, FORMAT)
        require(isinstance(event["id"], str) and event["id"] and event["id"] not in by_id and
                (event["parent"] is None or event["parent"] in by_id), FORMAT)
        by_id[event["id"]] = event
    restored, covered, replayed = [], set(), 0
    for (tag, ref), key in zip(offered, keys):
        payload = seen[key]["payload"]
        require(set(payload) == {"id", "name", "mode", "state", "root", "path", "preferences", "episode", "exit",
                                 "assistance"} and payload["mode"] == "jumble_off" and payload["root"] == {"kind": "solved"},
                FORMAT)
        path = target(payload["path"])
        require(path["format"] == ident("replay-path") and path["payload"]["root"] == root and
                path["payload"]["events"] == events, FORMAT)
        require(tag != "head" or payload["path"]["content"]["sha256"] == digest_of["session/checkpoint-paths.json"],
                FORMAT)
        at, chain = path["payload"]["head"], []
        require(at == "" or at in by_id, FORMAT)
        while at:
            chain.insert(0, by_id[at])
            at = by_id[at]["parent"]
        covered.update(event["id"] for event in chain)
        moves, ends = [], []
        for event in chain:
            recipe = target(event["value"])
            require(recipe["format"] == ident("recipe"), FORMAT)  # the golden packages replay recipe events
            _, word = reference.expand(recipe["payload"]["steps"])
            require(recipe["payload"]["primitive_count"] == int_text(len(word)) and
                    recipe["payload"]["expansion"] == digest_ref("recipe_expansion", i32_bytes(word)), DIGEST)
            moves.extend(word)
            ends.append(len(moves))
        replayed += len(moves)
        charge("archive_replay", replayed)
        final, prefixes = reference.replay(moves)
        for event, start, end in zip(chain, [0] + ends, ends):
            require(event["pre"] == {"role": "labels", "sha256": prefixes[start]} and
                    event["post"] == {"role": "labels", "sha256": prefixes[end]}, REPLAY)
        final = i32_bytes(final)
        require(labels[f"session/checkpoints/{tag}.labels"] == final and
                payload["state"]["digest"] == digest_ref("labels", final), REPLAY)
        require(tag != "head" or manifest["head_state_hash"] == sha(final), REPLAY)
        require(target(payload["preferences"])["format"] == ident("preferences"), FORMAT)
        restored.append({"id": payload["id"], "head": path["payload"]["head"], "state_sha256": sha(final),
                         "preferences_sha256": payload["preferences"]["content"]["sha256"]})
    require(covered == set(by_id), FORMAT)  # every event lies on an offered path
    require(target(session["preferences"])["format"] == ident("preferences"), FORMAT)
    conversion = target(manifest["conversion"])
    require(conversion["format"] == ident("migration"), FORMAT)
    goal = conversion["payload"].get("target")
    return {"head_event_id": manifest["head_event_id"], "head_state_sha256": manifest["head_state_hash"],
            "checkpoints": restored, "session_preferences_sha256": session["preferences"]["content"]["sha256"],
            "conversion_target_sha256": goal["content"]["sha256"] if goal else None,
            "closure_records": len(seen), "blob_carriers": len(blobs), "replayed_letters": replayed}


def main():
    vectors = json.loads((HERE / "vectors.json").read_bytes())
    examples = json.loads((HERE / "examples.json").read_bytes())
    check_inventory()
    count = 0
    for vector in vectors["json"]:
        raw = canonical(vector["given"], vector["profile"])
        equal(raw.hex(), vector["expected_hex"], vector["id"] + " bytes")
        equal(sha(raw), vector["expected_sha256"], vector["id"] + " digest")
        if vector["profile"] == "new":
            equal(read_canonical(raw), vector["given"], vector["id"] + " read")
        count += 1
    for vector in vectors["invalid_json"]:
        equal(outcome(lambda: read_canonical(vector["given"].encode("utf-8"))), vector["expected"], vector["id"])
        count += 1
    for vector in vectors["numbers"]:
        value = vector["given"]
        equal(outcome(lambda: exact_value(value)), vector["expected"], vector["id"])
        if vector["expected"] == "accepted":
            raw = canonical(value)
            equal(raw.hex(), vector["expected_hex"], vector["id"] + " bytes")
            equal(sha(raw), vector["expected_sha256"], vector["id"] + " digest")
        count += 1
    big = "1" + "0" * 6773 + "7"
    equal(int_value(big), 10 ** 6774 + 7, "6775-digit arithmetic")
    equal(int_text(int_value(big)), big, "6775-digit round trip")
    equal(sha(canonical({"int": big})), vectors["large_integer"]["expected_sha256"], "6775-digit bytes")
    equal(len(canonical({"int": big})), vectors["large_integer"]["expected_bytes"], "6775-digit byte count")
    equal(outcome(lambda: int_value("1" * 10000)), "accepted", "integer digit limit")
    equal(outcome(lambda: int_value("1" * 10001)), SIZE, "integer digit overflow")
    count += 4
    for vector in vectors["gzip"]:
        raw = vector["given"].encode("utf-8")
        packed = pack_gzip(raw)
        equal(packed.hex(), vector["expected_hex"], vector["id"] + " bytes")
        equal(sha(packed), vector["expected_sha256"], vector["id"] + " digest")
        equal(unpack_gzip(packed), raw, vector["id"] + " strict read")
        equal(gzip.decompress(packed), raw, vector["id"] + " independent gzip read")
        equal(outcome(lambda: unpack_gzip(packed + packed)), CANON, "no concatenated members")
        count += 1
    for size in (65535, 65536, 250000, 250001):
        raw = b"a" * size
        equal(unpack_gzip(pack_gzip(raw)), raw, f"gzip boundary {size}")
        equal(gzip.decompress(pack_gzip(raw)), raw, f"gzip decoder {size}")
    equal(set(vectors["limits"]), set(LIMITS), "every shared limit has vectors")
    for row, vector in vectors["limits"].items():
        equal(vector["given"], [LIMITS[row][1], LIMITS[row][1] + 1], row + " boundary values")
        equal([outcome(lambda n=n: charge(row, n)) for n in vector["given"]], vector["expected"], row)
        count += 2
    specimen = examples["records"]["manual_flow"]
    for vector in vectors["unknown_fields"]:
        record = copy.deepcopy(specimen)
        record.update(vector["given"])
        for field in vector.get("remove", []):
            del record[field]
        equal(outcome(lambda: record_header(record)), vector["expected"], vector["id"])
        if vector["expected"] == "accepted":
            equal(read_canonical(canonical(record)), record, "unknown optional retained")
        count += 1
    for name, record in examples["records"].items():
        record_header(record)
        block = record["provenance"]
        provenance(block)
        equal(read_canonical(canonical(block)), block, name + " round trip")
        equal(sha(canonical(block)), vectors["provenance"][name]["expected_sha256"], name + " digest")
        equal(outcome(lambda: policy_use(block, block["block"]["form"], set())), PROVENANCE, name + " no grant")
        if name.startswith("imported"):
            source = examples["sources"]["legacy_recipe"].encode("utf-8")
            equal(block["legacy"]["source"], digest_ref("bytes", source), name + " source")
            equal(block["depends"], {"kind": "not_recorded"}, name + " unknown history")
        contract_text = examples["fixture_contracts"][block["block"]["form"]].encode("utf-8")
        equal(block["block"]["contract"]["source"], digest_ref("source", contract_text), name + " contract")
        missing = copy.deepcopy(block)
        del missing["profile"]
        equal(outcome(lambda: provenance(missing)), PROVENANCE, "missing profile stays unknown")
        changed = copy.deepcopy(block)
        changed["block"]["form"] = "unknown_form"
        equal(outcome(lambda: provenance(changed)), PROVENANCE, "unknown form")
        if block["block"]["form"] == "flow_static":
            changed["block"]["form"] = "flow_runtime"
            equal(outcome(lambda: provenance(changed)), "accepted", "runtime flow no run")
        count += 1
    wrong = copy.deepcopy(specimen["provenance"])
    wrong["producer"] = {"kind": "program", "module": digest_ref("source", b"fixture"), "entry": "main"}
    equal(outcome(lambda: provenance(wrong)), PROVENANCE, "program cannot have none profile")
    word = vectors["word"]
    raw = encode_ia16(word["given"])
    equal(raw.hex(), word["expected_hex"], "ia16 bytes")
    equal(sha(raw), word["expected_sha256"], "ia16 digest")
    equal(decode_ia16(raw), [tuple(pair) for pair in word["given"]], "ia16 round trip")
    # Cross a real chunk boundary without storing a million-byte fixture.
    letters = [(0, 0)] * (LIMITS["ia16_chunk_letters"][1] + 1) + [(599, 59)]
    raw = encode_ia16(letters)
    chunks = word_chunks(raw)
    equal([len(c) for c in chunks], word["chunk_expected_bytes"], "ia16 chunk boundary")
    equal(read_chunks(chunks, len(letters), sha(raw), "order-a", "order-a"), letters, "ia16 chunk replay")
    equal(outcome(lambda: read_chunks(chunks, len(letters), sha(raw), "order-a", "order-b")),
          "KN1:input.identity_mismatch", "ia16 order pin")
    equal(outcome(lambda: read_chunks(chunks, len(letters) - 1, sha(raw), "a", "a")), CANON, "ia16 length")
    equal(outcome(lambda: decode_ia16(struct.pack("<H", 36000))), CANON, "ia16 range")
    equal(outcome(lambda: decode_ia16(b"\0")), CANON, "ia16 width")
    count += 6
    reference = RetainedReference()
    for vector in vectors["retained_words"]:
        record = copy.deepcopy(examples["empty_signed_word"])
        recipes = {}
        case = vector["given"]["case"]
        if case in {"empty_recipe", "empty_recipe_step"}:
            recipe = {"steps": [] if case == "empty_recipe" else [{"kind": "word", "moves": []}],
                      "primitive_count": "0", "expansion": digest_ref("recipe_expansion", b"")}
            ref = synthetic_ref("recipe", case)
            recipes[ref["content"]["sha256"]] = recipe
            record["payload"] = {"alphabet": "signed_generators", "start": record["payload"]["start"], "recipe": ref}
        elif case == "nonzero_length":
            record["payload"]["length"] = "1"
        elif case == "nonempty_digest":
            record["payload"]["letters"] = digest_ref("recipe_expansion", i32_bytes([1]))
        moves = []
        equal(outcome(lambda: moves.extend(signed_word(record, lambda ref: recipes[ref["content"]["sha256"]], reference))),
              vector["expected"], vector["id"])
        if vector["expected"] == "accepted":
            equal(len(moves), vector["expected_length"], vector["id"] + " literal length")
        count += 1
    for vector in vectors["conversion"]:
        result = conversion_result(reference, vector["given"])
        equal(result, vector["expected"], vector["id"])
        count += 1
    word_recipe = vectors["conversion"][1]["given"]
    _, forward = reference.expand(word_recipe)
    left, _ = reference.replay(forward)
    right, _ = reference.replay(list(reversed(forward)))
    require(left != right, "chronology vector must not commute")
    equal(sha(i32_bytes([1, -1])), "b15348c8f462384c01e83b6d499c6faf3f96808f5aa07c6bab4b65b36b4445d4",
          "oracle README expansion digest")
    for vector in vectors["durable"]:
        equal(durable_result(vector["given"], reference), vector["expected"], vector["id"])
        count += 1
    for vector in vectors["dispatch"]:
        equal(dispatch(vector["given"]), vector["expected"], vector["id"])
        count += 1
    for vector in vectors["graphs"]:
        equal(outcome(lambda: graph_admission(vector["given"])), vector["expected"], vector["id"])
        count += 1
    stored_cases = json.loads((ROOT / "work/experiments/language-lx/envelope-facts/DISPLAY.json").read_bytes())["collision_examples"]
    stored_cases = {item["outcome"]: item["facts"] for item in stored_cases}
    covered_collisions = set()
    for vector in vectors["evidence"]:
        if "stored_collision" in vector:
            collision = vector["stored_collision"]
            equal(vector["stored_facts"], stored_cases[collision], vector["id"] + " stored experiment tuple")
            for source, target in (("shape", "shape"), ("job", "own_job")):
                equal(vector["given"][target], stored_cases[collision][source], vector["id"] + " fixture binding")
            covered_collisions.add(collision)
        equal(evidence_result(vector["given"]), vector["expected"], vector["id"])
        count += 1
    equal(covered_collisions, set(stored_cases), "all four stored collisions covered")
    for vector in vectors["evidence_pairs"]:
        result = {}
        for query in ("N", "E"):
            answer = evidence_result({**vector["given"], "query": query})
            result[query] = {key: answer[key] for key in ("row", "word", "code")}
        equal(result, vector["expected"], vector["id"])
        count += 1
    for vector in vectors["relations"]:
        equal(relation_result(vector["given"], examples), vector["expected"], vector["id"])
        count += 1
    for vector in vectors["aliases"]:
        try:
            result = alias_result(vector["given"])
        except Refusal as error:
            result = str(error)
        equal(result, vector["expected"], vector["id"])
        count += 1
    for vector in vectors["pins"]:
        try:
            result = pin_result(**vector["given"])
        except Refusal as error:
            result = str(error)
        equal(result, vector["expected"], vector["id"])
        count += 1
    for vector in vectors["storage"]:
        given = vector["given"]
        result = {"source_graph": outcome(lambda: charge("record_decoded", given["graph_bytes"])),
                  "expanded_word": outcome(lambda: charge("bundle_decoded", 2 * int_value(given["letters"])))}
        equal(result, vector["expected"], vector["id"])
        count += 1
    for vector in vectors["durable_invalid"]:
        equal(outcome(lambda: durable_result(vector["given"], reference)), vector["expected"], vector["id"])
        count += 1
    for vector in vectors["playback"]:
        try:
            result = playback_result(vector["given"])
        except Refusal as error:
            result = str(error)
        equal(result, vector["expected"], vector["id"])
        count += 1
    for family, check in (("playback_from", playback_from_result), ("playback_trace", playback_trace_result),
                          ("playback_arguments", playback_argument_result), ("playback_storage", playback_storage_result),
                          ("picks", pick_result), ("editions", editions_result), ("obligations", obligation_result)):
        for vector in vectors[family]:
            try:
                result = check(vector["given"])
            except Refusal as error:
                result = str(error)
            equal(result, vector["expected"], vector["id"])
            count += 1
    golden, cases = {}, []
    for vector in vectors["path_b_packages"]:
        given = vector["given"]
        package = path_b_package(given["fixture"], given["raw_copy"], reference, examples, given.get("mutation"))
        if isinstance(vector["expected"], str):
            equal(outcome(lambda: path_b_read(package, reference)), vector["expected"], vector["id"])
        else:
            equal({"package_sha256": sha(package), "package_bytes": len(package)},
                  {key: vector["expected"][key] for key in ("package_sha256", "package_bytes")}, vector["id"] + " package")
            with zipfile.ZipFile(io.BytesIO(package)) as archive:
                equal(archive.testzip(), None, vector["id"] + " independent ZIP read")
            golden.setdefault(given["fixture"], []).append(path_b_read(package, reference))
            equal(golden[given["fixture"]][-1], vector["expected"]["result"], vector["id"])
            cases.append((given["fixture"], given["raw_copy"]))
        count += 1
    equal(sorted(cases), [("abandoned-branch", False), ("abandoned-branch", True), ("fresh-session", False),
                          ("fresh-session", True)], "golden Path B packages with raw_copy off and on")
    for fixture, results in golden.items():
        equal(results[0], results[1], fixture + " restores the same with raw_copy on and off")
    evidence_text = (ROOT / "docs/architecture/1.0/language/architecture/06-evidence.md").read_bytes().decode("utf-8")
    e1 = evidence_text.split("**E1,", 1)[1].split("**E2,", 1)[0]
    require("(exact computation, independent checker)" not in e1 and
            all(word in e1 for word in ("unqualified", "KN6.2", "KN6.5", "macro.py", "replay_word.py")), FORMAT)
    replay_grades = [re.search(r"\(exact computation, ([^)]+)\)", line).group(1)
                     for line in e1.splitlines() if line.startswith("| ") and "exact computation" in line]
    equal(replay_grades, ["producer", "producer"], vectors["e1"]["id"])
    equal(replay_grades[0] + "; unqualified for KN6", vectors["e1"]["expected"], vectors["e1"]["id"])
    count += 1
    print(f"format vectors: PASS ({count} vectors; source/reference and synthetic only; no files written)")


if __name__ == "__main__":
    main()
