"""Exact neutral operations and their explicit constituent continuation schedule."""

from .meter import limb_weight


def calculate(fn, args, charge, inverses):
    def visit():
        charge("visit", 1)

    def arithmetic(op, a, b):
        charge(op, limb_weight(a, b))
        if op == "add":
            return a + b
        if op == "sub":
            return a - b
        if op == "mul":
            return a * b
        return a % b

    def compare(a, b):
        charge("compare", limb_weight(a, b))
        return (a > b) - (a < b)

    def equal(a, b):
        if type(a) is bool or type(b) is bool:
            charge("bool_compare", 1)
            return type(a) is type(b) and a == b
        if type(a) is int and type(b) is int:
            return compare(a, b) == 0
        if type(a) is str and type(b) is str:
            for x, y in zip(a, b):
                visit()
                charge("scalar_compare", 1)
                if x != y:
                    return False
            return compare(len(a), len(b)) == 0
        if isinstance(a, dict) and isinstance(b, dict):
            if not equal(sorted(a), sorted(b)):
                return False
            pairs = [(a[k], b[k]) for k in sorted(a)]
        elif isinstance(a, (list, tuple)) and isinstance(b, (list, tuple)):
            if compare(len(a), len(b)) != 0:
                return False
            pairs = zip(a, b)
        else:
            charge("scalar_compare", 1)
            return a is b
        for x, y in pairs:
            visit()
            if not equal(x, y):
                return False
        return True

    def collection(values):
        result = []
        for value in values:
            visit()
            result.append(value)
        return result

    if fn == "identity":
        return args[0]
    if fn in ("field", "index"):
        return args[0][args[1]]
    if fn == "singleton":
        return [args[0]]
    if fn == "length":
        return len(args[0])
    if fn == "eq":
        return equal(*args)
    if fn == "gt":
        return compare(*args) > 0
    if fn == "add":
        return arithmetic("add", *args)
    if fn == "sum":
        result = 0
        for value in args[0]:
            visit()
            result = arithmetic("add", result, value)
        return result
    if fn == "count_equal":
        result = 0
        for value in args[0]:
            visit()
            if equal(value, args[1]):
                result = arithmetic("add", result, 1)
        return result
    if fn in ("argmax_first", "argmin_first"):
        best = 0
        for i, value in enumerate(args[0]):
            visit()
            if i:
                order = compare(value, args[0][best])
                if (order > 0 if fn == "argmax_first" else order < 0):
                    best = i
        return best
    if fn == "rank_desc_indices":
        ranked = []
        for i, value in enumerate(args[0]):
            visit()
            j = len(ranked)
            while j:
                visit()
                old = ranked[j - 1]
                order = compare(value, args[0][old])
                if order < 0 or (order == 0 and compare(i, old) >= 0):
                    break
                j -= 1
            ranked.insert(j, i)
        return ranked
    if fn in ("append", "concat"):
        return collection(list(args[0]) + ([args[1]] if fn == "append" else list(args[1])))
    if fn == "inverse":
        return collection(inverses[x] for x in reversed(args[0]))
    if fn == "zip":
        return collection([a, b] for a, b in zip(*args))
    if fn == "indexed_points":
        return collection([i, value] for i, value in enumerate(args[0]))
    if fn == "point":
        return collection([list(args)])
    if fn == "mod_dot":
        result = 0
        for a, b in zip(args[0], args[1]):
            visit()
            result = arithmetic("add", result, arithmetic("mul", a, b))
        return arithmetic("mod", result, args[2])
    if fn == "combine_corrections":
        result = []
        for word, coefficient in zip(*args):
            visit()
            for _ in range(coefficient):
                for letter in word:
                    visit()
                    result.append(letter)
        return result
    if fn == "linear_solve_mod":
        matrix, rhs, modulus = args
        rows = [list(row) + [v] for row, v in zip(matrix, rhs)]
        columns, pivot_row, pivots = len(matrix[0]), 0, []
        for column in range(columns):
            pivot = None
            for row in range(pivot_row, len(rows)):
                visit()
                if compare(rows[row][column], 0) != 0:
                    pivot = row
                    break
            if pivot is None:
                continue
            rows[pivot_row], rows[pivot] = rows[pivot], rows[pivot_row]
            factor = rows[pivot_row][column]
            for inverse in range(1, modulus):
                residue = arithmetic("mod", arithmetic("mul", factor, inverse), modulus)
                if compare(residue, 1) == 0:
                    break
            for j in range(column, columns + 1):
                visit()
                rows[pivot_row][j] = arithmetic("mod", arithmetic("mul", rows[pivot_row][j], inverse), modulus)
            for row in range(len(rows)):
                if row == pivot_row:
                    continue
                visit()
                factor = rows[row][column]
                if compare(factor, 0) == 0:
                    continue
                for j in range(column, columns + 1):
                    visit()
                    product = arithmetic("mul", factor, rows[pivot_row][j])
                    rows[row][j] = arithmetic("mod", arithmetic("sub", rows[row][j], product), modulus)
            pivots.append((pivot_row, column))
            pivot_row += 1
        result = collection(0 for _ in range(columns))
        for row, column in pivots:
            visit()
            result[column] = rows[row][-1]
        return result
    raise ValueError("unknown neutral calculation: " + fn)


def self_test():
    from .kernel import Kernel
    charges = []
    def run(fn, *args):
        return calculate(fn, args, lambda kind, weight: charges.append((kind, weight)), Kernel().fixture["inverses"])
    assert run("rank_desc_indices", [2, 6, 6]) == [1, 2, 0]
    assert run("argmax_first", [2, 6, 6]) == 1
    assert run("argmin_first", [2, 1, 1]) == 1
    assert run("linear_solve_mod", [[1, 0], [0, 1]], [0, 3], 5) == [0, 3]
    assert run("linear_solve_mod", [[1, 1], [1, 0]], [0, 1], 2) == [1, 1]
    assert run("combine_corrections", [["c"], ["d"]], [0, 3]) == ["d", "d", "d"]
    assert run("mod_dot", [1, 2], [3, 1], 5) == 0
    assert run("eq", {"b": [1, True]}, {"b": [1, True]})
    assert not run("eq", True, 1)
    charges.clear()
    assert run("add", 2**64, 1) == 2**64 + 1
    assert charges == [("add", 2)]
