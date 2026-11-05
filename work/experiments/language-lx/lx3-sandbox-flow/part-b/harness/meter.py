"""Deterministic logical charges and cooperative host events, not CPU timing."""

from .common import canonical, load

COUNTERS = ("steps", "kernel_cost", "memory", "output")


class Stop(Exception):
    def __init__(self, outcome, code, reason, path, loop_index=None):
        super().__init__(code or reason)
        self.outcome, self.code = outcome, code
        self.stop = {"reason": reason, "run_id": path.split("/")[0],
                     "path": path, "loop_index": loop_index}


class Meter:
    def __init__(self, limits, host=None):
        self.profile = load("profile.json")
        self.limits = limits
        self.host = host or {}
        self.used = dict.fromkeys(COUNTERS, 0)

    def admit(self, run_id):
        path = run_id + "/admission"
        if self.limits is None:
            raise Stop("refused", "KN7:input.budget_not_declared", "before admission", path)
        if set(self.limits) != set(COUNTERS) or any(type(v) is not int or v < 0 for v in self.limits.values()):
            raise ValueError("invalid budget declaration")
        if any(self.limits[k] > v for k, v in self.host.get("ceiling", {}).items()):
            raise Stop("refused", "KN7:resource_limit.budget_ceiling", "before admission", path)

    def interrupt(self, path, loop_index=None):
        if self.host.get("event") == "cancel" and self.host["at"] == path:
            raise Stop("cancelled", "KN7:cancelled.by_user", "by_user", path, loop_index)

    def charge(self, counter, amount, path, loop_index=None):
        if type(amount) is not int or amount < 0:
            raise ValueError("charges must be nonnegative integers")
        if self.used[counter] + amount > self.limits[counter]:
            raise Stop("cut_off", "KN7:budget." + counter, "budget", path, loop_index)
        self.used[counter] += amount

    def continuation(self, path, weight=1, loop_index=None):
        self.interrupt(path, loop_index)
        self.charge("steps", weight, path, loop_index)

    def kernel(self, amount, path):
        self.charge("kernel_cost", amount, path + ":before_kernel")

    def bind(self, data, path, *, logical_bytes=None, snapshot=False):
        size = logical_bytes if logical_bytes is not None else (32 if snapshot else len(canonical(data)))
        self.charge("memory", size, path + ":before_memory")

    def output(self, data, path):
        self.charge("output", len(canonical(data)), path + ":before_output")

    def payload(self):
        return {"units": self.profile["units"], "granted": self.limits,
                "used": dict(self.used), "children": []}


def limb_weight(*values):
    return max(1, (max((abs(x).bit_length() for x in values), default=0) + 63) // 64)


def self_test():
    m = Meter(dict(steps=1, kernel_cost=2, memory=7, output=2))
    m.admit("r1")
    m.continuation("r1/body/0:before_step")
    try:
        m.kernel(3, "r1/body/0")
    except Stop as stop:
        assert stop.code == "KN7:budget.kernel_cost"
        assert m.used == dict(steps=1, kernel_cost=0, memory=0, output=0)
    else:
        raise AssertionError("failed kernel charge admitted")
    m.bind([2, 6, 6], "r1/body/0")
    assert m.used["memory"] == 7
    m.output("", "r1/body/0")
    assert m.used["output"] == 2
    c = Meter(dict.fromkeys(COUNTERS, 0), {"event": "cancel", "at": "r1/x"})
    try:
        c.continuation("r1/x")
    except Stop as stop:
        assert stop.code == "KN7:cancelled.by_user" and c.used["steps"] == 0
    assert limb_weight(2**64) == 2
