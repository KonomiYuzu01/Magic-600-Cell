"""Recompute LP1 draft accounting examples. Standard library only; no writes."""

import sys

sys.dont_write_bytecode = True

import hashlib
import json
from pathlib import Path


HERE = Path(__file__).resolve().parent
DIMENSIONS = ("steps", "kernel_cost", "memory", "output")


class Stop(Exception):
    def __init__(self, code, position, scope="root", outcome="cut_off", live=None):
        self.code, self.position, self.scope, self.outcome = code, position, scope, outcome
        self.live = live


def meter(name, limits):
    return {"name": name, "limits": limits, "used": dict.fromkeys(DIMENSIONS, 0),
            "live": 0, "reserved": 0}


def scalar_text(value):
    if any(0xD800 <= ord(char) <= 0xDFFF for char in value):
        raise ValueError("surrogate in Text")
    return value


def canonical(value):
    # PS1.2 retained JSON, used only as this fixture's declared output profile.
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True,
                      allow_nan=False).encode("utf-8")


def integer_size(value):
    return 1 + max(1, (abs(value).bit_length() + 7) // 8)


def stream_key(seed_hex, path):
    seed = bytes.fromhex(seed_hex)
    if len(seed) != 32:
        raise ValueError("seed must have 32 bytes")
    data = b"C600-LP1-RNG\0" + seed + len(path).to_bytes(4, "big")
    for segment in path:
        encoded = scalar_text(segment).encode("utf-8")
        data += len(encoded).to_bytes(4, "big") + encoded
    return hashlib.sha256(data).digest(), len(data)


class Model:
    """Abstract continuations, not a parser, IR format, kernel or product executor."""

    def __init__(self, given, tariffs):
        self.given, self.tariffs = given, tariffs
        self.root = meter("root", dict(given["limits"]))
        self.active = [self.root]
        self.owners = {}
        self.frames = {"root": None}
        self.charges, self.children = [], []
        self.values, self.coverage, self.generators = {}, {}, {}

    def charge(self, at, kernel=0, memory=0, output=0):
        event = [at, 0, 0, 0, 0, 0]  # path, steps, kernel, memory, output, release
        self.charges.append(event)
        for dim, amount in (("steps", 1), ("kernel_cost", kernel),
                            ("memory", memory), ("output", output)):
            for scope in self.active:
                current = scope["live"] if dim == "memory" else scope["used"][dim]
                if current + amount > scope["limits"][dim]:
                    position = {"at": at, "phase": dim, "scope": scope["name"]}
                    source = self.given.get("source_map", {}).get(at, self.given.get("source"))
                    if source is not None:
                        position["source"] = source
                    raise Stop("KN7:budget." + dim, position, scope["name"], live=self.root["live"])
            if dim == "steps":
                event[1] = 1
                for scope in self.active:
                    scope["used"][dim] += 1
        # Secondary requests commit together, after all three have passed.
        event[2:5] = [kernel, memory, output]
        for scope in self.active:
            scope["used"]["kernel_cost"] += kernel
            scope["used"]["output"] += output
            scope["live"] += memory
            scope["reserved"] += memory
            scope["used"]["memory"] = max(scope["used"]["memory"], scope["live"])

    def own(self, frame, name, size):
        key = (frame, name)
        if key in self.owners:
            raise ValueError("duplicate live owner: " + str(key))
        for scope in self.active:
            scope["reserved"] -= size
            if scope["reserved"] < 0:
                raise ValueError("owner exceeds admitted reservation")
        self.owners[key] = {"size": size, "meters": list(self.active)}

    def cursor_name(self, key):
        return key[1] if key[0] == "root" else key[0] + ":" + key[1]

    def bind_cursor(self, key, cursor):
        self.owners[key]["generator"] = cursor
        self.generators[self.cursor_name(key)] = cursor

    def lookup(self, frame, name):
        while frame is not None:
            if (frame, name) in self.owners:
                return (frame, name)
            frame = self.frames[frame]
        raise ValueError("missing owner: " + name)

    def release(self, size, scopes=None, record=True):
        for scope in self.active if scopes is None else scopes:
            if scopes is None:
                scope["reserved"] -= size
                if scope["reserved"] < 0:
                    raise ValueError("release exceeds unowned reservation")
            scope["live"] -= size
            if scope["live"] < 0:
                raise ValueError("negative live charge")
        if record:
            self.charges[-1][5] += size

    def drop(self, key, record=True):
        owner = self.owners.pop(key)
        self.release(owner["size"], owner["meters"], record)

    def close_frame(self, frame, keep=None, target=None):
        transferred = 0
        for key in list(self.owners):
            if key[0] != frame:
                continue
            if key[1] == keep:
                owner = self.owners.pop(key)
                new_key = (self.frames[frame], target)
                if new_key in self.owners:
                    raise ValueError("return overwrites a live owner")
                self.owners[new_key] = owner
                if "generator" in owner:
                    self.generators.pop(self.cursor_name(key))
                    self.generators[self.cursor_name(new_key)] = owner["generator"]
                transferred = owner["size"]
            else:
                self.drop(key)
        return transferred

    def call(self, op, at, parent):
        frame = at + "/frame"
        size = 16 + 8 * op.get("slots", 0)
        self.charge(at + "/call", memory=size)
        self.frames[frame] = parent
        self.own(frame, "#frame", size)
        self.run(op["body"], at + "/", frame)
        self.charge(at + "/return")
        return self.close_frame(frame, op.get("returns"), op.get("as"))

    def child(self, op, at, frame):
        self.charge(at + "/child")
        effective = {}
        for dim in DIMENSIONS:
            remaining = [s["limits"][dim] - (s["live"] if dim == "memory" else s["used"][dim])
                         for s in self.active]
            effective[dim] = min(op["limits"][dim], *remaining)
        child = meter(at, effective)
        self.active.append(child)
        outcome, code, position, transferred = "completed", None, None, 0
        propagate = None
        try:
            transferred = self.call(op, at + "/run", frame)
        except Stop as stop:
            outcome, code, position = stop.outcome, stop.code, stop.position
            if stop.scope != at:
                propagate = stop
            # No return of an unadmitted value. Unwind is not a charged operation.
            self.release(child["reserved"], record=False)
            for key, owner in list(self.owners.items()):
                if any(s is child for s in owner["meters"]):
                    self.drop(key, record=False)
        else:
            # Only moved result owners can survive. Ancestors already include them.
            for owner in self.owners.values():
                if any(s is child for s in owner["meters"]):
                    child["live"] -= owner["size"]
                    owner["meters"] = [s for s in owner["meters"] if s is not child]
        finally:
            self.active.pop()
        self.children.append({"at": at, "requested": op["limits"], "effective": effective,
                              "outcome": outcome, "code": code, "stop": position,
                              "used": child["used"], "returned_memory": transferred,
                              "live_after_return": child["live"]})
        if propagate is not None:
            raise propagate

    def run(self, trace, prefix="", frame="root"):
        t = self.tariffs
        for op in trace:
            kind, at = op["op"], prefix + op["at"]
            if kind in ("display_region", "display_branch"):
                if self.given.get("execution_contract") != "lp1-exec-region-draft-1":
                    raise ValueError("display region needs its own execution identity")
            if kind == "display_if" and self.given.get("execution_contract") == "lp1-exec-region-draft-1":
                raise ValueError("form-1 display_if cannot be relabelled as form 2")
            if kind in ("tick", "assert", "flow_check"):
                self.charge(at)
                if kind == "assert" and not op["value"]:
                    raise Stop("LG4:input.pointwise_failed", {"at": at, "phase": "effect",
                               "scope": self.active[-1]["name"]}, self.active[-1]["name"], "error")
                if kind == "flow_check" and not op["value"]:
                    raise ValueError("fixtures must not select a reserved flow refusal")
            elif kind == "alloc":
                self.charge(at, memory=op["bytes"])
                self.own(frame, op["name"], op["bytes"])
            elif kind == "alias":
                source = self.owners[self.lookup(frame, op["source"])]
                size = source["size"]
                self.charge(at, memory=size)
                self.own(frame, op["name"], size)
                if "generator" in source:
                    self.bind_cursor((frame, op["name"]), dict(source["generator"]))
            elif kind == "drop":
                key = self.lookup(frame, op["name"])
                self.charge(at)
                self.drop(key)
            elif kind == "call":
                self.call(op, at, frame)
            elif kind == "child":
                self.child(op, at, frame)
            elif kind == "display_region":
                scope = at + "/scope"
                self.charge(at + "/display_enter")
                self.frames[scope] = frame
                self.run(op["body"], at + "/", scope)
                self.charge(at + "/display_exit")
                self.close_frame(scope, op.get("returns"), op.get("as"))
            elif kind == "loop":
                for index in range(op["count"] + 1):
                    self.charge(at + "/test:" + str(index))
                    if index < op["count"]:
                        self.run(op["body"], at + "/body:" + str(index) + "/", frame)
            elif kind in ("branch", "display_if", "display_branch"):
                self.charge(at)
                branch = "then" if op["condition"] else "else"
                self.run(op[branch], at + "/" + branch + "/", frame)
            elif kind == "kernel":
                size, scratch = op.get("result_bytes", 0), op.get("scratch", 0)
                cost = t["kernel_base"] + op["items"] * t["kernel_per_item"]
                self.charge(at, kernel=cost, memory=size + scratch)
                if size:
                    self.own(frame, op["name"], size)
                self.release(scratch)
            elif kind in ("add", "mul"):
                a, b = int(op["a"]), int(op["b"])
                bits = [max(1, abs(v).bit_length()) for v in (a, b)]
                limbs = [(n + t["limb_bits"] - 1) // t["limb_bits"] for n in bits]
                cost = (sum(limbs) * t["add_per_limb"] if kind == "add" else
                        limbs[0] * limbs[1] * t["mul_per_limb_pair"])
                bound_bits = max(bits) + 1 if kind == "add" else sum(bits)
                bound = 1 + (bound_bits + 7) // 8
                self.charge(at, kernel=cost, memory=bound)
                value = a + b if kind == "add" else a * b
                self.own(frame, op["name"], integer_size(value))
                self.release(bound - integer_size(value))
                self.values[at] = value
            elif kind == "text_compare":
                a, b = scalar_text(op["a"]), scalar_text(op["b"])
                self.charge(at, kernel=(len(a) + len(b)) * t["text_per_scalar"])
                self.values[at] = (a > b) - (a < b)
            elif kind == "text_decode":
                data = bytes.fromhex(op["hex"])
                self.charge(at, kernel=len(data) * t["decode_per_byte"], memory=4 * len(data))
                try:
                    value = scalar_text(data.decode("utf-8", errors="strict"))
                except (UnicodeDecodeError, ValueError):
                    raise Stop("LG4:input.text_scalar_invalid", {"at": at, "phase": "effect",
                               "scope": self.active[-1]["name"]}, self.active[-1]["name"], "error") from None
                size = 4 * len(value)
                self.own(frame, op["name"], size)
                self.release(4 * len(data) - size)
                self.values[at] = value
            elif kind == "emit":
                size = len(canonical(op["value"]))
                self.charge(at, kernel=size * t["encode_per_byte"], memory=size, output=size)
                self.release(size)
                self.values[at] = canonical(op["value"]).hex()
            elif kind == "rng_init":
                key, size = stream_key(op["seed"], op["path"])
                self.charge(at, kernel=size * t["rng_init_per_byte"], memory=41)
                self.own(frame, op["name"], 41)
                self.bind_cursor((frame, op["name"]), {"key": key.hex(), "counter": 0})
            elif kind == "rng_draw":
                generator = self.owners[self.lookup(frame, op["generator"])]["generator"]
                self.charge(at, kernel=t["rng_block"], memory=33)
                if generator["counter"] == 2**64:
                    raise Stop("LG4:resource_limit.generator_exhausted", {"at": at, "phase": "effect",
                               "scope": self.active[-1]["name"]}, self.active[-1]["name"], "error")
                digest = hashlib.sha256(bytes.fromhex(generator["key"]) +
                                        generator["counter"].to_bytes(8, "big")).digest()
                value = int.from_bytes(digest, "big")
                generator["counter"] += 1
                self.own(frame, op["name"], integer_size(value))
                self.release(33 - integer_size(value))
                self.values[at] = digest.hex()
            elif kind == "stream":
                self.charge(at + "/open", kernel=t["stream_setup"], memory=op["cursor_bytes"])
                self.own(frame, op["name"], op["cursor_bytes"])
                self.coverage[at] = 0
                for start in range(0, op["length"], op["chunk_size"]):
                    length = min(op["chunk_size"], op["length"] - start)
                    cost = length * (t["expand_per_letter"] + t["replay_per_letter"])
                    scratch = length * op["letter_bytes"]
                    self.charge(at + "/chunk:" + str(start), kernel=cost, memory=scratch)
                    self.coverage[at] += length
                    self.release(scratch)
            else:
                raise ValueError("unknown abstract operation: " + kind)

    def result(self):
        outcome, code, position = "completed", None, None
        stopped_live = None
        try:
            self.run(self.given["trace"])
        except Stop as stop:
            outcome, code, position = stop.outcome, stop.code, stop.position
            stopped_live = stop.live
        before = self.root["live"] if stopped_live is None else stopped_live
        for key in list(self.owners):
            self.drop(key, record=False)
        # Reservations of an admitted operation ending in an error are abandoned too.
        self.release(self.root["reserved"], record=False)
        return {"outcome": outcome, "code": code, "stop": position, "used": self.root["used"],
                "live_before_cleanup": before, "live_after_cleanup": self.root["live"],
                "charges": self.charges, "values": self.values, "coverage": self.coverage,
                "generators": self.generators, "children": self.children}


def first_difference(expected, actual, path="result"):
    if type(expected) is not type(actual):
        return f"{path}: type differs"
    if isinstance(expected, dict):
        if expected.keys() != actual.keys():
            return f"{path}: fields differ"
        for key in expected:
            difference = first_difference(expected[key], actual[key], path + "." + key)
            if difference:
                return difference
    elif isinstance(expected, list):
        if len(expected) != len(actual):
            return f"{path}: length {len(expected)} != {len(actual)}"
        for index, (left, right) in enumerate(zip(expected, actual)):
            difference = first_difference(left, right, f"{path}[{index}]")
            if difference:
                return difference
    elif expected != actual:
        return f"{path}: expected {expected!r}, computed {actual!r}"
    return None


def boundary_result(given):
    """Small rule models over declared inputs, not a source checker or renderer."""
    if given["kind"] == "source_encoding_diagnostic":
        if given["edition"] != "supported-fixture":
            raise ValueError("encoding fixture needs a supported edition")
        data = bytes.fromhex(given["hex"])
        try:
            data.decode("utf-8", errors="strict")
        except UnicodeDecodeError as error:
            return {"outcome": "rejected", "diagnostic": {
                "code": "LG1:syntax.invalid_encoding", "form": "diagnostic",
                "context": {"raw_input_digest": hashlib.sha256(data).hexdigest(),
                            "byte_start": error.start, "byte_end": error.end}},
                "decoded_source": None, "host_range": None}
        raise ValueError("encoding fixture needs malformed UTF-8")
    if given["kind"] == "source_range":
        try:
            check_source(given["range"], bytes.fromhex(given["hex"]))
        except ValueError:
            return {"valid": False}
        return {"valid": True}
    if given["kind"] == "exactness_diagnostic":
        codes = []
        for case in given["cases"]:
            if case["value_type"] == "ApproxBool" or case["display_control"] or case.get("qualifier_erasure", False):
                codes.append("LG2:type.exactness_violation")
            elif case["value_type"] == "Approx":
                codes.append("LG2:type.exact_required")
            else:
                raise ValueError("unsupported exactness refusal fixture")
        return {"checker_codes": codes, "editor_codes": list(codes)}
    if given["kind"] == "follow_pick":
        if given["picked_bindings"] != given["current_bindings"]:
            return {"code": "EV3:stale.display_pick", "source_action": False}
        producer = hashlib.sha256(given["producing_source"].encode("utf-8")).digest()
        target = hashlib.sha256(given["target_source"].encode("utf-8")).digest()
        if producer != target:
            return {"code": "EV3:stale.source_link", "source_action": False}
        return {"code": None, "source_action": True}
    if given["kind"] == "straddler_publication":
        # Inputs declare certified full data and the ceiling comparison, not geometry or a tariff.
        if given["producer_budgets"] != "fit" or given["fixed_renderer_ceiling"] != "exceeded":
            raise ValueError("straddler fixture needs fitting budgets and an exceeded fixed ceiling")
        if given["admission"] not in ("standalone", "scheduled"):
            raise ValueError("unknown publication admission state")
        accepted = given["admission"] == "scheduled"
        return {"code": "EV3:resource_limit.display_plan_ceiling",
                "form": "stop" if accepted else "refusal",
                "receipt": "failed" if accepted else "refused",
                "prior_A": "cancelled(superseded(by: B.command))" if accepted else "pending",
                "invocation": given["invocation"], "new_admission": False, "installed": False,
                "full_set": given["full_set"]}
    raise ValueError("unknown boundary fixture kind")


def check_source(source, data):
    if source["source"] != hashlib.sha256(data).hexdigest():
        raise ValueError("source digest mismatch")
    data.decode("utf-8", errors="strict")
    if not 0 <= source["start"] <= source["end"] <= len(data):
        raise ValueError("source range outside fixture bytes")
    data[:source["start"]].decode("utf-8")
    data[:source["end"]].decode("utf-8")


def main():
    fixtures = json.loads((HERE / "vectors.json").read_bytes())
    if fixtures["contract"] != "lp1-exec-draft-1" or fixtures["tariff_status"] != "test-only; unmeasured":
        raise ValueError("wrong contract or unmarked test tariffs")
    if fixtures["output_profile"] != "PS1.2 retained JSON (fixture only)":
        raise ValueError("unsupported fixture byte profile")
    findings = json.loads((HERE.parent / "closure" / "findings.json").read_bytes())["findings"]
    required = {(fid, tag) for fid, finding in findings.items() for tag in finding["owners"].get("p4", [])}
    reconciled = json.loads((HERE.parent / "closure" / "reconciliation-findings.json").read_bytes())["findings"]
    required |= {(fid, tag) for fid in ("S2-02", "S2-03", "S2-04", "X-H6")
                 for tag in reconciled[fid]["owners"]["p7-lang"]}
    required |= {("LANG-01", "raw_encoding_diagnostic"), ("LANG-01", "no_fabricated_source_range"),
                 ("MC-05", "unowned_reservation_unwind"), ("S4-01", "full_straddler_fixed_ceiling"),
                 ("S4-02", "registry_pick_precedence")}
    covered, names = set(), set()
    for vector in fixtures["vectors"]:
        if vector["id"] in names:
            raise ValueError("duplicate vector ID")
        names.add(vector["id"])
        covered.update(tuple(pair) for pair in vector["covers"])
        if "source" in vector["given"] or "source_map" in vector["given"]:
            data = vector["given"]["source_text"].encode("utf-8")
            if "source" in vector["given"]:
                check_source(vector["given"]["source"], data)
            for source in vector["given"].get("source_map", {}).values():
                check_source(source, data)
        if vector["given"].get("execution_contract", fixtures["contract"]) not in (
                "lp1-exec-draft-1", "lp1-exec-region-draft-1"):
            raise ValueError("unknown fixture execution identity")
        for dimension in DIMENSIONS:
            limit = vector["given"]["limits"][dimension]
            if type(limit) is not int or limit < 0:
                raise ValueError("limits must be finite nonnegative integers")
        actual = Model(vector["given"], fixtures["tariffs"]).result()
        difference = first_difference(vector["expected"], actual)
        if difference:
            raise ValueError(vector["id"] + ": " + difference)
    for vector in fixtures["boundary_vectors"]:
        if vector["id"] in names:
            raise ValueError("duplicate vector ID")
        names.add(vector["id"])
        covered.update(tuple(pair) for pair in vector["covers"])
        difference = first_difference(vector["expected"], boundary_result(vector["given"]))
        if difference:
            raise ValueError(vector["id"] + ": " + difference)
    if required - covered:
        raise ValueError("missing finding tags: " + repr(sorted(required - covered)))
    print(f"contract vectors: PASS ({len(fixtures['vectors'])} traces, "
          f"{len(fixtures['boundary_vectors'])} boundary vectors; {len(required)} required finding tags; test tariffs only)")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OverflowError) as error:
        sys.exit("contract vectors: FAIL: " + str(error))
