"""Trusted per-run services given to a candidate's adapter."""

from .common import canonical, digest, load, sha, thaw
from .meter import Meter, Stop
from .policy import Policy
from .result import payload_hash


class RunContext:
    def __init__(self, case, group, run, host, bindings, form, variant, previous=None,
                 host_variant="default", loss_callback=None):
        self.case, self.group, self.run = case, group, run
        self.program = host.program
        self.host, self.kernel, self.policy = host, host.kernel, Policy()
        self.bindings, self.form, self.variant = bindings, form, variant
        self.previous, self.host_variant = previous, host_variant
        self.grants = bindings["effective_grants"][run["profile"]]
        self.admitted = False
        self.meter = Meter(case.get("limits", load("profile.json")["default_budgets"]), case.get("host", {}))
        host.begin(run)
        self.before = host.snapshot()
        self.coverage = {"examined": [], "total": [i for i, op in enumerate(run["body"]) if op["op"] != "return"], "complete": False}
        self.display, self.executions, self.dynamic_attempts = None, [], []
        self.cache_diagnostics = []
        self.kernel_calls = []
        self.layer, self.position = "executor", run["id"] + "/admission"
        self.loss_callback = loss_callback
        self.cache = {}
        self.cache_reads = 0

    def kernel_call(self, row, values, path):
        args = [v.data for v in values]
        if row == "preview_outcome":
            args = [self.host.preview_outcome(values[0])]
        if row == "batch_check":
            self.coverage = {"examined": [], "total": list(range(len(args[0]))), "complete": False}
            result = []
            retained = 0
            for i, word in enumerate(args[0]):
                self.meter.interrupt(path + ":chunk/" + str(i), i)
                self.meter.kernel(self.kernel.cost("word_check", [word]), path + ":chunk/" + str(i))
                staged = result + [self.kernel.call("word_check", [word])]
                size = len(canonical(staged))
                self.meter.charge("memory", size - retained, path + ":chunk/" + str(i) + ":before_memory")
                result, retained = staged, size
                self.kernel_calls.append({"path": path + ":chunk/" + str(i), "row": "word_check",
                                          "args_digest": digest([word]), "result_digest": digest(staged[-1])})
                self.coverage["examined"].append(i)
            if not result:
                self.meter.bind([], path)
            return tuple(result)
        self.meter.kernel(self.kernel.cost(row, args), path)
        key = digest({"fixture": digest(self.kernel.fixture), "row": row, "args": args})
        actual = self.kernel.call(row, args)
        if self.host_variant != "default":
            # Validation against the immutable fixture oracle is deliberately
            # synthetic. No speed benefit or cache performance is claimed.
            entry = self.cache.get(key)
            if entry is not None and entry != actual:
                self.cache_diagnostics.append("LG4:input.cache_entry_mismatch")
                entry = None
            if entry is None:
                self.cache[key] = actual
            actual = self.cache[key]
            self.cache_reads += 1
            if self.host_variant == "evicted" and self.cache_reads == 1:
                self.cache.clear()
        self.kernel_calls.append({"path": path, "row": row, "args_digest": digest(args), "result_digest": digest(actual)})
        return actual

    def admit(self):
        self.meter.admit(self.run["id"])
        self.admitted = True
        if self.case.get("host", {}).get("event") == "deadline":
            raise Stop("not_run", None, "corpus_issue", self.run["id"] + "/admission")
        if self.case.get("host", {}).get("event") == "runtime_loss":
            if self.loss_callback is None:
                raise ValueError("runtime loss requires the matrix parent")
            self.loss_callback(self)

    def bind_grants(self, grants):
        """Pin the mechanism's selected subset before execution/admission."""
        if self.admitted or self.executions or self.meter.used["steps"]:
            raise ValueError("effective grants must be fixed before admission")
        if not set(grants) <= set(self.bindings["effective_grants"][self.run["profile"]]):
            raise ValueError("effective grants exceed the frozen declaration")
        self.grants = sorted(set(grants))

    def executed(self, path, data):
        """Record actual executed fragment bytes against the frozen source map."""
        mapping = self.bindings["source_map"][self.case["id"]][path]
        self.executions.append({"path": path, "approved_sha256": mapping["sha256"],
                                "executed_sha256": sha(data), "inside_approval": sha(data) == mapping["sha256"]})

    def refuse(self, code, *, operation="admission", layer="candidate", path=None):
        self.layer = layer
        path = path or self.position
        self.host.refusal(operation, code, layer, path)
        raise Stop("refused", code, "policy", path)

    def check(self, operation, values=(), *, labels, read_state, **facts):
        code = self.policy.decide(operation, form=self.form, profile=self.run["profile"],
                                  author=self.run["author"], boundary_class=self.run["boundary_class"],
                                  invocation=self.run["invocation"], provenance=self.host.metadata(values),
                                  labels=labels, grants=self.grants, read_state=read_state, **facts)
        if code:
            self.refuse(code, operation=operation, layer="shared_policy")

    def finish(self, value=None, stop=None, reason=None):
        outcome, code = ("completed", None) if stop is None else (stop.outcome, stop.code)
        context = {"layer": self.layer}
        if stop is not None:
            stop.stop["run_id"] = self.run["id"]
        if stop is not None and outcome == "not_run" and reason is None:
            reason = stop.stop["reason"]
        if reason:
            context["reason"] = reason
        if outcome == "completed" and stop is None:
            self.coverage["complete"] = self.coverage["examined"] == self.coverage["total"]
        consumed = list(self.host.consumed)
        if self.previous is not None:
            consumed.append({"kind": "previous_run", "sha256": self.previous})
        binding = digest({"candidate": digest(self.bindings), "form": self.form, "variant": self.variant,
                          "grants": self.grants, "consumed": consumed})
        neutral = next(k["neutral_sha256"] for k in load("keys.json")["keys"]
                       if k["case_id"] == self.case["id"] and k["run_id"] == self.run["id"])
        identity = self.kernel.fixture["identity"]
        identities = {"neutral_sha256": neutral, "binding_sha256": binding,
                      "run_key": digest({"neutral_sha256": neutral, "binding_sha256": binding}),
                      "fixture": digest(self.kernel.fixture),
                      **{key: identity[key] for key in ("model", "menu", "level", "contract")}}
        words = []
        if value is not None:
            if value.type == "Word":
                words = [thaw(value.data)]
            elif value.type == "Words":
                words = thaw(value.data)
            elif value.type == "Record" and "word" in value.data:
                words = [thaw(value.data["word"])]
        exact = {"outcome": outcome, "code": code, "context": context, "input_identities": identities,
                 "coverage": self.coverage, "budgets": self.meter.payload(),
                 "value": None if value is None else {"type": value.type, "value": thaw(value.data)},
                 "words": words, "certificates": [], "checker_reports": [],
                 "traces": self.host.events, "stop": None if stop is None else stop.stop}
        observer = {"before": self.before, "after": self.host.snapshot(), "shell": self.host.shell,
                    "trap_calls": self.host.trap_calls, "denials": self.host.denials,
                    "kernel_calls": self.kernel_calls,
                    "approval": {"executions": self.executions, "dynamic_attempts": self.dynamic_attempts},
                    "consumed": consumed, "provenance_blocks": {key: thaw(v) for key, v in sorted(self.host.blocks.items())},
                    "runtime_loss_observed": False}
        return {"exact": exact, "sha256": payload_hash(exact), "display": self.display,
                "observer": observer, "host": {"cache_diagnostics": self.cache_diagnostics,
                    "diagnostics": {"sha256": sha(b""), "planted_data": False}}, "repeat": 0}


def self_test():
    from .host import Host
    case = load("allowed-tools.json")["cases"][0]
    bindings = {"effective_grants": {"solving": []}, "source_map": {}}
    h = Host(case["program"], "static_labels", {})
    c = RunContext(case, "allowed-tools", case["program"]["runs"][0], h, bindings,
                   "static_labels", "source_bound")
    c.admit()
    record = c.finish()
    assert record["observer"]["before"] == record["observer"]["after"]
    assert record["exact"]["budgets"]["used"]["steps"] == 0
    assert len(record["exact"]["input_identities"]["run_key"]) == 64
    words = h.make("Words", [["a"], ["c"]], "test")
    batch = c.kernel_call("batch_check", [words], "r1/test")
    assert isinstance(batch, tuple) and len(batch) == 2
    assert c.meter.used["memory"] == len(canonical(batch))
    before = c.meter.used["memory"]
    assert c.kernel_call("batch_check", [h.make("Words", [], "empty")], "r1/empty") == ()
    assert c.meter.used["memory"] == before + 2
