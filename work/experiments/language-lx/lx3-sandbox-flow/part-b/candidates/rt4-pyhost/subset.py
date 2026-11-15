"""Approved Python AST subset and a finite, conservative static label checker.

This duplicates front-end work: names, allowed effects, module closure, branch
joins and loop fixed points. It does not execute the neutral notation or select
policy. Persisted provenance is a symbolic input checked at the library boundary.
"""

import ast
from dataclasses import dataclass

from harness.policy import STATE_ROWS


METHOD_ARITY = {"input": 1, "literal": 3, "step": 2, "done": 1,
                "kernel": 5, "calc": 5, "iterate": 4, "call": 4,
                "workspace_get": 2, "workspace_put": 3, "preview": 3,
                "pick": 4, "emit": 2, "publish": 3, "retain": 2,
                "ambient": 4, "dynamic": 2, "result": 3, "stop": 2}
NODES = (ast.Module, ast.FunctionDef, ast.arguments, ast.arg, ast.Assign,
         ast.Name, ast.Load, ast.Store, ast.Expr, ast.Call, ast.Attribute,
         ast.Constant, ast.List, ast.Dict, ast.If, ast.For, ast.Tuple,
         ast.Return, ast.Pass, ast.UnaryOp, ast.USub)


class SubsetError(ValueError):
    pass


def constant(node):
    return ast.literal_eval(node)


def method(node):
    return node.func.attr if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute) else None


def approve(source):
    tree = ast.parse(source, filename="rt4-approved-program", mode="exec")
    functions = {node.name: node for node in tree.body if isinstance(node, ast.FunctionDef)}
    if len(functions) != len(tree.body):
        raise SubsetError("only function declarations at module scope")
    for node in ast.walk(tree):
        if not isinstance(node, NODES):
            raise SubsetError("host construct outside approved subset")
        if isinstance(node, ast.FunctionDef):
            if node not in tree.body or not node.name.startswith(("m_", "r_")) or node.decorator_list or node.returns or node.type_params:
                raise SubsetError("unsupported function declaration")
            args = node.args
            if (args.posonlyargs or args.kwonlyargs or args.defaults or args.kw_defaults
                    or args.vararg or args.kwarg or not args.args or args.args[0].arg != "lib"):
                raise SubsetError("unsupported function arguments")
            if any(not arg.arg.startswith("v_") or arg.annotation for arg in args.args[1:]):
                raise SubsetError("unsupported parameter")
        if isinstance(node, ast.Name):
            if node.id != "lib" and not node.id.startswith("v_") and node.id not in functions:
                raise SubsetError("unbound host name")
            if isinstance(node.ctx, ast.Store) and not node.id.startswith("v_"):
                raise SubsetError("cannot replace library or functions")
        if isinstance(node, ast.Assign):
            if len(node.targets) != 1 or not isinstance(node.targets[0], ast.Name):
                raise SubsetError("only local value assignment")
        if isinstance(node, ast.Attribute):
            if not isinstance(node.value, ast.Name) or node.value.id != "lib" or node.attr not in METHOD_ARITY:
                raise SubsetError("host attributes are unavailable")
            if not any(isinstance(parent, ast.Call) and parent.func is node for parent in ast.walk(tree)):
                raise SubsetError("library methods cannot escape as values")
        if isinstance(node, ast.Call):
            if (not isinstance(node.func, ast.Attribute) or node.keywords
                    or len(node.args) != METHOD_ARITY.get(node.func.attr)):
                raise SubsetError("unsupported call")
            if node.func.attr == "call":
                if not isinstance(node.args[2], ast.Name) or node.args[2].id not in functions:
                    raise SubsetError("unknown module")
        if isinstance(node, ast.For) and (node.orelse or method(node.iter) != "iterate"):
            raise SubsetError("loops require the metered domain iterator")
    return functions


def closure(functions, entry):
    reached, pending = set(), [entry]
    while pending:
        name = pending.pop()
        if name in reached:
            continue
        reached.add(name)
        pending.extend(node.args[2].id for node in ast.walk(functions[name])
                       if isinstance(node, ast.Call) and method(node) == "call")
    return reached


def has_preview(functions, entry):
    return any(isinstance(node, ast.Call) and method(node) == "preview"
               for name in closure(functions, entry) for node in ast.walk(functions[name]))


@dataclass(frozen=True)
class Info:
    labels: frozenset = frozenset()
    reads: bool = False
    references: frozenset = frozenset()
    origin: str = "derived"


EMPTY = Info()


def join(*items):
    return Info(frozenset().union(*(x.labels for x in items)), any(x.reads for x in items),
                frozenset().union(*(x.references for x in items)),
                items[0].origin if items and all(x.origin == items[0].origin for x in items) else "derived")


class Analysis:
    def __init__(self, functions, inputs, profile, transparent):
        self.functions, self.inputs = functions, inputs
        self.profile, self.transparent = profile, transparent
        self.outputs, self.gates = {}, {}
        self.stack = []

    def gate(self, path, operation, info, **facts):
        if path in self.gates:
            old = self.gates[path]
            info = join(info, old[1])
        self.gates[path] = (operation, info, facts)

    def produced(self, path, info):
        if self.profile == "research":
            info = join(info, Info(frozenset({"research"})))
        self.outputs[path] = join(self.outputs.get(path, info), info)
        return Info(info.labels, info.reads, info.references | {path}, info.origin)

    def expression(self, node, env, control):
        if isinstance(node, ast.Name):
            return env[node.id]
        if isinstance(node, ast.List):
            return [self.expression(item, env, control) for item in node.elts]
        if not isinstance(node, ast.Call):
            return EMPTY
        name, args = method(node), node.args
        if name == "literal":
            return Info(references=frozenset({constant(args[2])}), origin="fixture")
        if name == "input":
            item = self.inputs[constant(args[0])]
            return Info(frozenset({"unknown"}) if item["origin"] == "unknown" else frozenset(),
                        references=frozenset({"input/" + constant(args[0])}), origin=item["origin"])
        if name in ("step", "done"):
            return EMPTY
        path = constant(args[0])
        if name in ("kernel", "calc", "call"):
            values = self.expression(args[3], env, control)
        elif name in ("preview", "publish"):
            values = [self.expression(arg, env, control) for arg in args[1:]]
        elif name in ("workspace_put", "pick"):
            values = [self.expression(args[2], env, control)]
        elif name in ("result", "emit", "dynamic", "iterate"):
            values = [self.expression(args[1], env, control)]
        else:
            values = []
        info = join(control, *values)
        operation = {"result": "return", "iterate": "for"}.get(name, name)
        facts = {}
        if name == "kernel":
            operation = constant(args[2])
            if operation == "setup":
                facts = {"manual_target": values[0].origin == "manual", "fixed_entry": True}
        if name == "ambient":
            facts = {"resource": constant(args[2])}
        self.gate(path, operation, info, **facts)
        if name == "kernel":
            if operation in STATE_ROWS:
                info = join(info, Info(frozenset({"state"}), True))
            if operation in ("setup", "caps"):
                info = Info(info.labels, info.reads, info.references,
                            "fixed_entry" if operation == "setup" else "fixed_object_set")
        elif name == "calc":
            if constant(args[1]) in ("Word", "Words", "Target", "Targets") and constant(args[2]) not in self.transparent:
                info = join(info, Info(frozenset({"computed"})))
        elif name == "workspace_get":
            info = join(info, Info(references=frozenset({path})))
        elif name == "preview":
            info = join(info, Info(frozenset({"state"}), True))
        elif name == "call":
            info = join(info, self.function(args[2].id, values, control))
        return self.produced(path, info)

    def block(self, body, env, control):
        returns = []
        for node in body:
            if isinstance(node, ast.Assign):
                env[node.targets[0].id] = self.expression(node.value, env, control)
            elif isinstance(node, ast.Expr):
                self.expression(node.value, env, control)
            elif isinstance(node, ast.Return):
                returns.append(self.expression(node.value, env, control))
            elif isinstance(node, ast.If):
                branch_control = join(control, self.expression(node.test, env, control))
                left, right = dict(env), dict(env)
                returns += self.block(node.body, left, branch_control)
                returns += self.block(node.orelse, right, branch_control)
                for key in left.keys() | right.keys():
                    env[key] = join(left.get(key, EMPTY), right.get(key, EMPTY))
                # Deliberately no stop-sensitive continuation repair.
            elif isinstance(node, ast.For):
                items = self.expression(node.iter, env, control)
                local = dict(env)
                local[node.target.elts[0].id] = EMPTY
                local[node.target.elts[1].id] = items
                while True:
                    before = dict(local)
                    returns += self.block(node.body, local, join(control, items))
                    for key in before.keys() | local.keys():
                        local[key] = join(before.get(key, EMPTY), local.get(key, EMPTY))
                    if before == local:
                        break
                for key in env:
                    env[key] = join(env[key], local[key])
        return returns

    def function(self, name, args=(), control=EMPTY):
        if name in self.stack:
            return join(control, *args)  # Recursive fixture has no returning path.
        node = self.functions[name]
        env = {arg.arg: value for arg, value in zip(node.args.args[1:], args)}
        self.stack.append(name)
        try:
            return join(*self.block(node.body, env, control))
        finally:
            self.stack.pop()
