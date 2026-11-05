"""Synthetic host effects and provenance. No OS resource is used by a trap."""

from dataclasses import dataclass

from .common import digest, freeze, load, thaw
from .kernel import Kernel


@dataclass(frozen=True)
class Value:
    type: str
    data: object
    block: object

    @property
    def labels(self):
        return frozenset(self.block["labels"])

    @property
    def read_state(self):
        return self.block["read_state"]


class Host:
    def __init__(self, program, form, source_pins):
        self.kernel = Kernel()
        self.fixture = self.kernel.fixture
        self.program, self.form, self.source_pins = program, form, source_pins
        self.session = thaw(self.fixture["session"])
        self.workspace = {}
        self.pending = thaw(self.fixture["pending_preview"])
        self.blocks, self.receipts = {}, {}
        self.events, self.shell, self.trap_calls = [], [], []
        self.plants = {c["plant"]["logical_resource"]: c for c in load("planted.json")["cases"]}
        self.run = None
        self.consumed, self.denials = [], []

    def begin(self, run):
        self.run = run
        self.events, self.shell, self.trap_calls = [], [], []
        self.consumed, self.denials = [], []

    def make(self, kind, data, entry, dependencies=(), *, labels=(), read_state=False,
             origin="derived", pick=None, declassifiers=()):
        """Trusted adapter API; interpreted program data never supplies a block."""
        parts = [digest(value.block) for value in dependencies]
        profiles = {self.run["profile"]}
        for value in dependencies:
            profiles.update(value.block["profiles"])
        block = freeze({"version": 1, "form": self.form, "source_pins": self.source_pins,
                        "value_digest": digest(data),
                        "module_pins": {n: digest(m) for n, m in self.program["modules"].items()},
                        "author": self.run["author"], "entry": entry,
                        "entry_digest": digest({"entry": entry, "source": self.source_pins}),
                        "profile": self.run["profile"], "profiles": sorted(profiles),
                        "dependencies": parts, "parts": parts, "pick": pick,
                        "declassifiers": list(declassifiers), "labels": sorted(labels),
                        "read_state": bool(read_state), "origin": origin,
                        "unknown": origin == "unknown" or any(v.block["unknown"] for v in dependencies)})
        self.blocks[digest(block)] = block
        return Value(kind, freeze(data), block)

    def input(self, name):
        item = self.program["inputs"][name]
        labels = ["unknown"] if item["origin"] == "unknown" else []
        return self.make(item["type"], item["value"], "input/" + name,
                         labels=labels, origin=item["origin"])

    def metadata(self, values):
        return {"valid": all(digest(v.block) in self.blocks for v in values),
                "unknown": any(v.block["unknown"] for v in values),
                "profiles": sorted({p for v in values for p in v.block["profiles"]})}

    def event(self, kind, data, value=None):
        event = {"kind": kind, **thaw(data)}
        if value is not None:
            event["provenance"] = thaw(value.block)
        self.events.append(event)

    def workspace_put(self, key, value):
        self.workspace[key] = value
        self.event("workspace_put", {"key": key, "value": value.data, "profile": self.run["profile"]}, value)

    def workspace_get(self, key):
        value = self.workspace[key]
        if value.block is None or value.block.get("version") != 1:
            value = self.make(value.type, value.data, "workspace/" + key,
                              origin="unknown", labels=["unknown"])
        self.consumed.append({"kind": "workspace", "key": key, "value": digest(value.data),
                              "provenance": digest(value.block), "parts": thaw(value.block["parts"])})
        return value

    def pick(self, options, event, kind, entry, *, labels, read_state):
        index = self.program["inputs"][event]
        if index["origin"] != "manual" or index["type"] != "Int":
            raise ValueError("pick needs a frozen manual input")
        chosen = options.data[index["value"]]
        fixed = options.block["origin"] in ("manual", "fixed_entry", "fixed_object_set")
        record = {"producer_digest": digest(options.block), "list_digest": digest(options.data),
                  "fixed_entry_or_object_set": fixed, "option_digest": digest(chosen),
                  "index": index["value"], "chosen": thaw(chosen), "event": event}
        self.consumed.append({"kind": "pick", "digest": digest(record)})
        value = self.make(kind, chosen, entry, [options], labels=labels, read_state=read_state,
                          pick=record, origin="fixed_entry" if fixed else "derived",
                          declassifiers=[{"id": "own_pick", "record_digest": digest(record)}] if fixed else [])
        self.event("pick", {"record": record}, value)
        return value

    def preview(self, word, target, entry, *, labels, read_state, control=()):
        capture = self.run["profile"] != "solving"
        check = self.kernel.word_check(word.data)
        outcome = {"code": None if capture or check["protected"] else "KN2:protection.piece_would_move",
                   "protected": bool(check["protected"])}
        public = {"acknowledged": True} if self.form == "capability_separation" else outcome
        dependencies = [word, target] + list(control)
        if self.form == "capability_separation":
            labels = {label for dependency in dependencies for label in dependency.labels}
            read_state = any(dependency.read_state for dependency in dependencies)
        else:
            labels = set(labels) | {"state"}
            read_state = True
        if self.run["profile"] == "research":
            labels = set(labels) | {"research"}
        value = self.make("Receipt", public, entry, dependencies, labels=labels, read_state=read_state)
        self.receipts[digest(value.block)] = freeze(outcome)
        observation = {"word": thaw(word.data), "target": target.data,
                       "destination": "capture" if capture else "solving_preview"}
        self.event("preview", observation, value)
        self.shell.append({**observation, "outcome": outcome, "provenance": thaw(value.block)})
        if not capture and outcome["code"] is None:
            self.pending = {"word": thaw(word.data), "target": target.data, "provenance": thaw(value.block)}
        return value

    def preview_outcome(self, receipt):
        # The candidate must ask policy before this row. The non-enforcing control
        # can observe the counterfactual even when the public receipt is constant.
        return self.receipts[digest(receipt.block)]

    def publish(self, exact, coordinates, value):
        plan = {"exact": {**thaw(exact), "source_digest": digest(self.program),
                           "input_digest": digest(self.fixture["session"])},
                "coordinates": thaw(coordinates)}
        self.event("publish", {"exact": plan["exact"]}, value)
        return plan

    def trap(self, resource, logical_resource):
        plant = self.plants[logical_resource]
        if plant["resource"] != resource:
            raise ValueError("trap resource mismatch")
        self.trap_calls.append({"resource": resource, "logical_resource": logical_resource})
        return plant["plant"]["data"]

    def snapshot(self):
        return {"session": thaw(self.session), "pending_preview": thaw(self.pending),
                "workspace": {key: {"type": value.type, "value": thaw(value.data),
                                     "provenance": thaw(value.block)}
                              for key, value in sorted(self.workspace.items())}}

    def refusal(self, operation, code, layer, path):
        state = self.snapshot()
        self.denials.append({"operation": operation, "code": code, "layer": layer,
                            "path": path, "before": state, "after": self.snapshot()})


def self_test():
    program = {"inputs": {"word": {"type": "Word", "value": ["a"], "origin": "manual"},
                           "target": {"type": "Target", "value": "c0", "origin": "manual"}}, "modules": {}}
    h = Host(program, "capability_separation", {"test": "source"})
    run = dict(id="r1", profile="solving", author="built_in")
    h.begin(run)
    before = h.snapshot()
    word, target = h.input("word"), h.input("target")
    a = h.preview(word, target, "r1/a", labels=[], read_state=False)
    bad = h.make("Word", ["c"], "bad")
    b = h.preview(bad, target, "r1/b", labels=[], read_state=False)
    assert a.data == b.data == {"acknowledged": True}
    assert not a.read_state and "state" not in a.labels
    assert h.preview_outcome(a)["code"] is None
    assert h.preview_outcome(b)["code"] == "KN2:protection.piece_would_move"
    assert h.snapshot()["session"] == before["session"]
    assert h.pending["word"] == ["a"]
    h.workspace_put("saved", word)
    h.begin(dict(run, id="r2", profile="research"))
    assert h.workspace_get("saved").block == word.block
    h.preview(word, target, "r2/a", labels=["research"], read_state=False)
    assert h.pending["word"] == ["a"] and h.shell[-1]["destination"] == "capture"
    assert h.trap("file", "file_probe") == "LX3_FILE_NEVER_RETURN"
    assert len(h.trap_calls) == 1
    assert Host(program, "static_labels", {}).workspace == {}
    h.workspace["missing_block"] = Value("Word", ("a",), None)
    unknown = h.workspace_get("missing_block")
    assert unknown.block["unknown"] and "unknown" in unknown.labels
    assert h.workspace["missing_block"].block is None
    h.program["inputs"]["pick0"] = {"type": "Int", "value": 0, "origin": "manual"}
    computed = h.make("Words", [["a"]], "computed", labels=["state", "computed"], read_state=True)
    selected = h.pick(computed, "pick0", "Word", "pick", labels=computed.labels, read_state=computed.read_state)
    assert selected.labels == computed.labels and selected.read_state
    assert not selected.block["declassifiers"]
