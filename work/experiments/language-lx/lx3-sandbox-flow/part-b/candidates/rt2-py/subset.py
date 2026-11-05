"""Approved Python grammar and conservative, branch-joining AST analysis.

No neutral evaluator, expected outcomes, corpus case names or VM label hooks.
The annotations are computed before CPython executes the checked tree.
"""

import ast
import json
from dataclasses import dataclass

from harness.common import validate_exact
from harness.policy import STATE_ROWS
from render import identifier


class SubsetError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise SubsetError(message)


def constant(node, kind=str):
    require(isinstance(node, ast.Constant) and type(node.value) is kind, "literal argument required")
    return node.value


def variable(node):
    require(isinstance(node, ast.Name) and node.id.startswith(("v_", "c_"))
            and node.id[2:] and all(c in "0123456789abcdef" for c in node.id[2:]), "value name required")
    return node.id


def call_name(node):
    require(isinstance(node, ast.Call) and isinstance(node.func, ast.Name)
            and not node.keywords, "only direct, positional helper calls")
    return node.func.id


# s: text literal; n: integer literal; v: value; a: value array; m: module.
SIGNATURES = {
    "h_input": "s", "h_literal": "sss", "h_truth": "v", "h_done": "s",
    "h_kernel": "sssaa", "h_calc": "sssaa", "h_call": "ssmaa",
    "h_condition": "sva", "h_iter": "ssva", "h_workspace_get": "sssa",
    "h_workspace_put": "ssva", "h_preview": "svva", "h_pick": "ssvsa",
    "h_emit": "sva", "h_publish": "svva", "h_return": "sva",
    "h_retain": "sna", "h_ambient": "ssssa", "h_dynamic": "sva",
    "h_stop": "sa", "h_tick": "sa",
}
ASSIGNMENTS = {"h_input", "h_kernel", "h_calc", "h_call", "h_condition",
               "h_workspace_get", "h_preview", "h_pick", "h_retain", "h_ambient"}
EFFECTS = {"h_done", "h_workspace_put", "h_emit", "h_publish", "h_dynamic", "h_stop", "h_tick"}


def value_expression(node):
    if isinstance(node, ast.Name):
        variable(node)
    else:
        require(call_name(node) == "h_literal" and len(node.args) == 3, "only sealed literal construction in expressions")
        constant(node.args[0])
        validate_exact(json.loads(constant(node.args[1])))
        constant(node.args[2])


def approved_tree(source):
    """Closed grammar: no attributes, imports, subscripts, eval, or reflection."""
    tree = ast.parse(source, filename="rt2-approved.py", mode="exec")
    functions, paths = {}, set()
    for node in tree.body:
        require(isinstance(node, ast.FunctionDef), "only function declarations at module scope")
        require(node.name.startswith(("m_", "r_")) and node.name[2:]
                and all(c in "0123456789abcdef" for c in node.name[2:]), "invalid function name")
        require(node.name not in functions and not node.decorator_list and node.returns is None
                and not node.type_params and node.type_comment is None, "unadorned unique functions only")
        args = node.args
        require(not (args.posonlyargs or args.vararg or args.kwarg or args.kwonlyargs or args.defaults
                     or args.kw_defaults), "simple parameters only")
        for arg in args.args:
            variable(ast.Name(id=arg.arg))
            require(arg.annotation is None and arg.type_comment is None, "no annotations")
        functions[node.name] = node

    def call(node):
        name = call_name(node)
        require(name in SIGNATURES, "helper not available")
        signature = SIGNATURES[name]
        require(len(node.args) == len(signature), "helper arity")
        for argument, shape in zip(node.args, signature):
            if shape in ("s", "n"):
                constant(argument, str if shape == "s" else int)
            elif shape == "v":
                value_expression(argument)
            elif shape == "a":
                require(isinstance(argument, ast.List), "value list required")
                for value in argument.elts:
                    value_expression(value)
            else:
                require(isinstance(argument, ast.Name) and argument.id.startswith("m_")
                        and argument.id in functions, "sealed module required")
        if name not in ("h_input", "h_literal", "h_truth", "h_done"):
            path = constant(node.args[0])
            require(path not in paths, "duplicate instruction path")
            paths.add(path)
        return name

    def body(statements):
        for node in statements:
            if isinstance(node, ast.Assign):
                require(len(node.targets) == 1 and node.type_comment is None, "single assignment only")
                name = variable(node.targets[0])
                helper = call(node.value)
                require(helper in ASSIGNMENTS, "invalid assignment helper")
                require(name.startswith("c_") == (helper == "h_condition"), "condition name contract")
            elif isinstance(node, ast.Expr):
                require(call(node.value) in EFFECTS, "invalid effect helper")
            elif isinstance(node, ast.Return):
                require(call(node.value) == "h_return", "metered return required")
            elif isinstance(node, ast.If):
                require(call(node.test) == "h_truth" and isinstance(node.test.args[0], ast.Name)
                        and node.test.args[0].id.startswith("c_"), "checked condition required")
                body(node.body)
                body(node.orelse)
            elif isinstance(node, ast.For):
                variable(node.target)
                require(not node.orelse and node.type_comment is None and call(node.iter) == "h_iter", "metered iteration only")
                body(node.body)
            else:
                require(isinstance(node, ast.Pass), "statement outside approved subset")
    for function in functions.values():
        body(function.body)
    return tree, functions, paths


@dataclass(frozen=True)
class Info:
    labels: frozenset = frozenset()
    read: bool = False


EMPTY = Info()
UNKNOWN = Info(frozenset({"unknown"}))
TOP = Info(frozenset({"unknown", "state", "research", "computed"}), True)


def join(*values):
    return Info(frozenset().union(*(value.labels for value in values)), any(value.read for value in values))


def merge_environments(left, right, names=None):
    return {name: join(left.get(name, EMPTY), right.get(name, EMPTY))
            for name in (set(left) | set(right) if names is None else names)}


class Analysis:
    def __init__(self, tree, functions, program, transparent):
        self.tree, self.functions, self.program = tree, functions, program
        self.transparent = set(transparent)
        self.marks, self.active = {}, set()
        self.dynamic = []
        self.graph = {name: {node.args[2].id for node in ast.walk(fn)
                             if isinstance(node, ast.Call) and isinstance(node.func, ast.Name)
                             and node.func.id == "h_call"} for name, fn in functions.items()}
        self.recursive = {name for name in functions if any(name in self.reachable(child)
                          for child in self.graph[name])}
        workspace = {}
        for run in program["runs"]:
            self.profile = run["profile"]
            _, workspace = self.function(identifier("r_", run["id"]), [], EMPTY, workspace)

    def reachable(self, name):
        seen, pending = set(), [name]
        while pending:
            current = pending.pop()
            if current not in seen:
                seen.add(current)
                pending.extend(self.graph[current])
        return seen

    def has_preview(self, run_id):
        return any(isinstance(node, ast.Call) and isinstance(node.func, ast.Name)
                   and node.func.id == "h_preview"
                   for name in self.reachable(identifier("r_", run_id)) for node in ast.walk(self.functions[name]))

    def value(self, node, env):
        if isinstance(node, ast.Name):
            require(node.id in env, "unbound value")
            return env[node.id]
        return EMPTY  # Already checked literal; the program cannot supply origin.

    def function(self, name, arguments, pc, workspace):
        if name in self.active:
            return TOP, workspace
        fn = self.functions[name]
        require(len(arguments) == len(fn.args.args), "module argument count")
        self.active.add(name)
        try:
            env = {arg.arg: info for arg, info in zip(fn.args.args, arguments)}
            _, result, workspace = self.block(fn.body, env, pc, dict(workspace))
            return result, workspace
        finally:
            self.active.remove(name)

    def annotate(self, path, use, result):
        old = self.marks.get(path, (EMPTY, EMPTY))
        self.marks[path] = (join(old[0], use), join(old[1], result))

    def operation(self, node, env, pc, workspace):
        name, args = node.func.id, node.args
        if name == "h_input":
            item = self.program["inputs"][constant(args[0])]
            return (UNKNOWN if item["origin"] == "unknown" else EMPTY), workspace
        if name == "h_done":
            return EMPTY, workspace
        path = constant(args[0])
        values = [self.value(v, env) for shape, arg in zip(SIGNATURES[name], args)
                  for v in (arg.elts if shape == "a" else [arg] if shape == "v" else [])]
        use = join(pc, *values)
        result = use
        if name == "h_kernel" and constant(args[2]) in STATE_ROWS:
            result = join(result, Info(frozenset({"state"}), True))
        elif name == "h_calc" and constant(args[1]) in ("Word", "Words", "Target", "Targets") and constant(args[2]) not in self.transparent:
            result = join(result, Info(frozenset({"computed"})))
        elif name == "h_call":
            arguments = [self.value(v, env) for v in args[3].elts]
            controls = join(pc, *(self.value(v, env) for v in args[4].elts))
            returned, workspace = self.function(args[2].id, arguments, controls, workspace)
            result = join(result, returned)
        elif name == "h_workspace_get":
            result = join(result, workspace.get(constant(args[2]), UNKNOWN))
        elif name == "h_preview":
            result = join(result, Info(frozenset({"state"}), True))
        elif name == "h_dynamic":
            self.dynamic.append(path)
        if self.profile == "research" and name != "h_workspace_get":
            result = join(result, Info(frozenset({"research"})))
        if name == "h_workspace_put":
            workspace[constant(args[1])] = result
        self.annotate(path, use, result)
        return result, workspace

    def block(self, statements, env, pc, workspace):
        returned = EMPTY
        for node in statements:
            if isinstance(node, ast.Pass):
                continue
            if isinstance(node, ast.If):
                condition = self.value(node.test.args[0], env)
                yes, a, wa = self.block(node.body, dict(env), join(pc, condition), dict(workspace))
                no, b, wb = self.block(node.orelse, dict(env), join(pc, condition), dict(workspace))
                env = merge_environments(yes, no, set(yes) & set(no))
                workspace = merge_environments(wa, wb)
                returned = join(returned, a, b)
                # Deliberately no stop-sensitive postdominator/termination repair.
            elif isinstance(node, ast.For):
                items, workspace = self.operation(node.iter, env, pc, workspace)
                old_names, initial = set(env), dict(env)
                while True:
                    local = dict(env)
                    local[node.target.id] = items
                    after, result, ws = self.block(node.body, local, join(pc, items), dict(workspace))
                    updated = merge_environments(initial, after, old_names - {node.target.id})
                    if node.target.id in initial:
                        updated[node.target.id] = initial[node.target.id]
                    next_workspace = merge_environments(workspace, ws)
                    returned = join(returned, result)
                    if updated == env and next_workspace == workspace:
                        break
                    env, workspace = updated, next_workspace
            else:
                result, workspace = self.operation(node.value, env, pc, workspace)
                if isinstance(node, ast.Assign):
                    env[node.targets[0].id] = result
                elif isinstance(node, ast.Return):
                    returned = join(returned, result)
        return env, returned, workspace
