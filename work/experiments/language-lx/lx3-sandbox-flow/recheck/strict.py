"""Additional preservation predicate; the frozen classifier remains unchanged."""

from harness.common import digest
from harness.host import Host
from harness.classify import classify as frozen_classify


class StrictHost(Host):
    """Checkpoint only trusted host mutations, checking before each one as well."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.checkpoint = self.snapshot()
        self.audit = {"checks": 0, "sanctioned": [], "violations": []}

    def inspect(self, at):
        live = self.snapshot()
        self.audit["checks"] += 1
        fields = [name for name in self.checkpoint if live[name] != self.checkpoint[name]]
        if fields:
            self.audit["violations"].append({"at": at, "fields": fields,
                "checkpoint_sha256": digest(self.checkpoint), "live_sha256": digest(live)})

    def advance(self, operation):
        self.checkpoint = self.snapshot()
        self.audit["sanctioned"].append(operation)

    def begin(self, run):
        self.audit = {"checks": 0, "sanctioned": [], "violations": []}
        self.inspect("before:begin")
        super().begin(run)
        self.advance("begin")

    def workspace_put(self, key, value):
        self.inspect("before:workspace_put")
        super().workspace_put(key, value)
        self.advance("workspace_put")

    def preview(self, *args, **kwargs):
        self.inspect("before:preview")
        result = super().preview(*args, **kwargs)
        self.advance("preview")
        return result

    def refusal(self, operation, code, layer, path):
        self.inspect("refusal:" + operation)
        super().refusal(operation, code, layer, path)


def install():
    """Called in each actual matrix child, including loss/shared-cache workers."""
    from harness import host, matrix, runtime
    host.Host = matrix.Host = StrictHost
    original = runtime.RunContext.finish
    peers = {}

    def finish(ctx, *args, **kwargs):
        ctx.host.inspect("finish")
        record = original(ctx, *args, **kwargs)
        # The excluded host envelope preserves exact payloads and consumed keys.
        record["host"]["strict_preservation"] = ctx.host.audit
        # matrix.worker retains only one of B12/shared's two exact-equal records.
        # Keep both audits on that record so the second host cannot disappear.
        audits = peers.setdefault(ctx.run["id"], [])
        audits.append(ctx.host.audit)
        record["host"]["strict_peer_preservation"] = audits
        return record

    runtime.RunContext.finish = finish


def violations(record):
    audits = record["host"]["strict_peer_preservation"]
    assert audits, "strict child wrapper missing"
    for audit in audits:
        assert audit["checks"] >= 2 and audit["sanctioned"][0] == "begin", "strict child wrapper missing"
    return [violation for audit in audits for violation in audit["violations"]]


def classify(case, group, record=None, **kwargs):
    baseline = frozen_classify(case, group, record, **kwargs)
    if record is None:
        return baseline
    failures = violations(record)
    if baseline["status"] == "not_run":
        return baseline
    if failures:
        return {"status": "state_changed" if group in ("must-refuse", "planted", "approval")
                else "wrong_value", "code": baseline["code"], "reason": "strict_preservation"}
    return baseline
