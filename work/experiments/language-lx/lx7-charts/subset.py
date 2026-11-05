"""LX-7 experiment subset. Parse only: never execute supplied Python code."""
import ast
from dataclasses import dataclass
from fractions import Fraction
import math
import struct

INPUTS = ("x", "y", "z", "w", "d")
OK, SINGULAR, NONFINITE = 0, 1, 2
COVERAGE = {OK: "point", SINGULAR: "singular_neighbourhood", NONFINITE: "nonfinite"}


class Refusal(ValueError):
    def __init__(self, code):
        self.code = code
        super().__init__(code)


def f32(value):
    try:
        return struct.unpack("<f", struct.pack("<f", value))[0]
    except OverflowError:
        return math.copysign(math.inf, value)


def bits(value):
    return struct.unpack("<I", struct.pack("<f", value))[0]


@dataclass(frozen=True)
class Program:
    # Operands are INPUTS, earlier t<number> registers, or float32 constants.
    instructions: tuple
    outputs: tuple


def admit(source):
    if len(source.encode("utf-8")) > 4096:
        raise Refusal("E_BOUNDS")
    try:
        tree = ast.parse(source)
    except (SyntaxError, ValueError, RecursionError):
        raise Refusal("E_SYNTAX") from None
    nodes = list(ast.walk(tree))
    if len(nodes) > 512:
        raise Refusal("E_BOUNDS")
    functions = {n.name for n in nodes if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef))}
    if any(isinstance(n, ast.Call) and isinstance(n.func, ast.Name)
           and n.func.id in functions for n in nodes):
        raise Refusal("E_RECURSION")
    if any(isinstance(n, ast.While) for n in nodes):
        raise Refusal("E_UNBOUNDED_LOOP")
    env = {name: name for name in INPUTS}
    instructions = []
    outputs = None

    def emit(op, *args):
        if len(instructions) >= 256:
            raise Refusal("E_BOUNDS")
        dest = "t" + str(len(instructions))
        instructions.append((dest, op, args))
        return dest

    def literal(node):
        if not isinstance(node, ast.Constant) or type(node.value) not in (int, float):
            raise Refusal("E_LITERAL")
        spelling = ast.get_source_segment(source, node)
        try:
            exact = Fraction(spelling)
            rounded = f32(float(exact))
        except (ValueError, OverflowError, ZeroDivisionError):
            raise Refusal("E_LITERAL") from None
        if not math.isfinite(rounded) or Fraction(rounded) != exact:
            raise Refusal("E_LITERAL")
        return rounded

    def expr(node, depth=0):
        if depth > 16:
            raise Refusal("E_BOUNDS")
        if isinstance(node, ast.Constant):
            return literal(node)
        if isinstance(node, ast.Name):
            if node.id not in env:
                raise Refusal("E_NAME")
            return env[node.id]
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.USub):
            return emit("neg", expr(node.operand, depth + 1))
        if isinstance(node, ast.BinOp):
            op = {ast.Add: "add", ast.Sub: "sub", ast.Mult: "mul", ast.Div: "div"}.get(type(node.op))
            if op is None:
                raise Refusal("E_OPERATION")
            return emit(op, expr(node.left, depth + 1), expr(node.right, depth + 1))
        if isinstance(node, ast.Call):
            if isinstance(node.func, ast.Name) and node.func.id == "abs" and len(node.args) == 1 and not node.keywords:
                return emit("abs", expr(node.args[0], depth + 1))
            raise Refusal("E_CALL")
        raise Refusal("E_SYNTAX")

    def statements(body, depth=0):
        nonlocal outputs
        if depth > 4:
            raise Refusal("E_BOUNDS")
        for node in body:
            if outputs is not None:
                raise Refusal("E_OUTPUT")
            if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
                name = node.targets[0].id
                if name in INPUTS or name in ("_", "abs", "exclude", "output", "range") or name.startswith("_"):
                    raise Refusal("E_NAME")
                env[name] = expr(node.value)
            elif isinstance(node, ast.For):
                call = node.iter
                if (node.orelse or not isinstance(node.target, ast.Name) or node.target.id != "_"
                        or not isinstance(call, ast.Call) or not isinstance(call.func, ast.Name)
                        or call.func.id != "range" or len(call.args) != 1 or call.keywords
                        or not isinstance(call.args[0], ast.Constant) or type(call.args[0].value) is not int):
                    raise Refusal("E_UNBOUNDED_LOOP")
                count = call.args[0].value
                if not 1 <= count <= 8:
                    raise Refusal("E_BOUNDS")
                for _ in range(count):
                    statements(node.body, depth + 1)
            elif isinstance(node, ast.Expr) and isinstance(node.value, ast.Call):
                call = node.value
                if not isinstance(call.func, ast.Name) or call.keywords:
                    raise Refusal("E_CALL")
                if call.func.id == "exclude" and len(call.args) == 2:
                    radius = literal(call.args[1])
                    if radius <= 0:
                        raise Refusal("E_LITERAL")
                    emit("exclude", expr(call.args[0]), radius)
                elif call.func.id == "output" and len(call.args) == 3 and depth == 0:
                    outputs = tuple(expr(arg) for arg in call.args)
                else:
                    raise Refusal("E_CALL")
            else:
                raise Refusal("E_SYNTAX")

    statements(tree.body)
    if outputs is None:
        raise Refusal("E_OUTPUT")
    return Program(tuple(instructions), outputs)


def evaluate(program, point, d):
    env = dict(zip(INPUTS, (f32(v) for v in (*point, d)), strict=True))
    if not all(math.isfinite(v) for v in env.values()):
        return NONFINITE, None

    def get(arg):
        return env[arg] if isinstance(arg, str) else arg

    for dest, op, args in program.instructions:
        values = [get(arg) for arg in args]
        a = values[0]
        if op == "exclude":
            if abs(a) <= values[1]:
                return SINGULAR, None
            continue
        if op == "add":
            value = a + values[1]
        elif op == "sub":
            value = a - values[1]
        elif op == "mul":
            value = a * values[1]
        elif op == "div":
            if values[1] == 0:
                return NONFINITE, None
            value = a / values[1]
        elif op == "neg":
            value = -a
        elif op == "abs":
            value = abs(a)
        else:
            raise AssertionError(op)
        env[dest] = f32(value)
        if not math.isfinite(env[dest]):
            return NONFINITE, None
    return OK, tuple(get(arg) for arg in program.outputs)
