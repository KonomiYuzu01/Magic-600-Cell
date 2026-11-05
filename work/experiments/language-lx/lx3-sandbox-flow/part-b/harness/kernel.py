"""Every frozen fake row; no policy, forms, host effects or mutable results."""

from .common import freeze, load, thaw


class Kernel:
    def __init__(self, fixture=None):
        self.fixture = freeze(load("fixture.json") if fixture is None else fixture)

    def cost(self, row, args=()):
        tariff = self.fixture["rows"][row]["cost"]
        if type(tariff) is int:
            return tariff
        if row == "batch_check":
            return sum(1 + len(word) for word in args[0])
        return 1 + len(args[0])

    def word_check(self, word):
        positions = list(range(len(self.fixture["session"]["labels"])))
        for letter in word:
            permutation = self.fixture["permutations"][letter]
            positions = [permutation[p] for p in positions]
        labels = [None] * len(positions)
        for source, destination in enumerate(positions):
            labels[destination] = self.fixture["session"]["labels"][source]
        moved = [i for i, p in enumerate(positions) if i != p]
        return freeze({"word": word, "end_labels": labels, "moved_slots": moved,
                       "protected": all(positions[i] == i for i in self.fixture["session"]["protected_slots"]),
                       "coverage": {"letters": len(word), "complete": True}})

    def call(self, row, args=()):
        signature = self.fixture["rows"][row]
        if len(args) != len(signature["args"]):
            raise ValueError("wrong kernel arity")
        if row == "snapshot":
            value = self.fixture["session"]["labels"]
        elif row == "word_check":
            value = self.word_check(args[0])
        elif row == "score":
            value = sum(v != i for i, v in enumerate(self.word_check(args[0])["end_labels"]))
        elif row == "setup":
            value = self.fixture["setups"][args[0]]
        elif row == "preview_outcome":
            value = {k: args[0][k] for k in ("code", "protected")}
        elif row == "batch_check":
            value = [self.word_check(word) for word in args[0]]
        else:
            value = self.fixture[row]
        return freeze(value)


def self_test():
    k = Kernel()
    for row, signature in k.fixture["rows"].items():
        args = {"word_check": [["a"]], "score": [["a"]], "setup": ["c1"],
                "preview_outcome": [{"code": None, "protected": True}],
                "batch_check": [[["a"], ["c"]]]}.get(row, [])
        assert k.cost(row, args) >= 1
        result = k.call(row, args)
        if isinstance(result, tuple):
            assert result is not args
    assert thaw(k.word_check(["a", "a"]))["moved_slots"] == []
    assert k.word_check(["D", "a"])["end_labels"] == (2, 0, 3, 1)
    assert k.call("score", [["a"]]) == 0
    assert not k.word_check(["c"])["protected"]
    for words in k.fixture["setups"].values():
        assert all(k.word_check(w)["protected"] for w in words)
    try:
        k.word_check(["a"])["protected"] = False
    except TypeError:
        pass
    else:
        raise AssertionError("mutable kernel result")
