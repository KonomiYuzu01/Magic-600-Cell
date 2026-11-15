"""Local labels-off variants. Original candidate/bench bytes are never edited.

Only the fixed workload is authorized. Empty ABI/provenance fields remain, as do
capability separation and its coarse read-state flag. This is no security claim.
"""

import ast
import importlib.util
from pathlib import Path
import sys

from support import PART_B, require, sha


def module(name, path, transform=None, extra=None):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    sys.modules[name] = result
    result.__dict__.update(extra or {})
    source = path.read_bytes()
    if transform:
        tree = transform(ast.parse(source))
        exec(compile(ast.fix_missing_locations(tree), path.name, "exec"), result.__dict__)
    else:
        spec.loader.exec_module(result)
    return result


class EmptyFlows(ast.NodeTransformer):
    """Retain RT1 type/effect traversal, erase every flow construction/join."""

    def visit_Call(self, node):
        if isinstance(node.func, ast.Name) and node.func.id in ("Flow", "join"):
            return ast.copy_location(ast.Name(id="EMPTY_FLOW", ctx=ast.Load()), node)
        return self.generic_visit(node)

    def visit_Module(self, node):
        self.generic_visit(node)
        for i, item in enumerate(node.body):
            if isinstance(item, ast.ClassDef) and item.name == "Flow":
                node.body.insert(i + 1, ast.parse("EMPTY_FLOW = Flow()\n").body[0])
                break
        return node


class NoExtraLabels(ast.NodeTransformer):
    """Erase RT4's state/computed tag arguments before their expressions run."""

    def visit_keyword(self, node):
        if node.arg == "extra_labels":
            node.value = ast.Tuple(elts=[], ctx=ast.Load())
        return self.generic_visit(node)

    def visit_Assign(self, node):
        if any(isinstance(t, ast.Name) and t.id == "computed" for t in node.targets):
            return ast.copy_location(ast.Pass(), node)
        return self.generic_visit(node)


def no_label_host():
    from harness.host import Host

    # Host.preview otherwise still computes observational label unions even in
    # capability mode. Replace just assignments to that local; retain receipts,
    # capabilities, read-state, immutable blocks, declassifiers and effects.
    class ClearLabels(ast.NodeTransformer):
        def visit_Assign(self, node):
            if any(isinstance(t, ast.Name) and t.id == "labels" for t in node.targets):
                node.value = ast.Tuple(elts=[], ctx=ast.Load())
            return self.generic_visit(node)

    import inspect
    import textwrap
    namespace = dict(Host.preview.__globals__)
    tree = ClearLabels().visit(ast.parse(textwrap.dedent(inspect.getsource(Host.preview))))
    exec(compile(ast.fix_missing_locations(tree), "lx1-labels-off-preview", "exec"), namespace)

    class LabelFreeHost(Host):
        preview = namespace["preview"]

        def __init__(self, *args):
            self.read_seen = False
            super().__init__(*args)

        def make(self, kind, data, entry, dependencies=(), *, labels=(), read_state=False, **extra):
            dependencies = tuple(dependencies)
            return super().make(kind, data, entry, dependencies, labels=(),
                                read_state=read_state or self.read_seen or any(v.read_state for v in dependencies),
                                **extra)

    return LabelFreeHost


def load(candidate, mode):
    folder = PART_B / "candidates" / candidate
    sys.path.insert(0, str(folder))
    off = mode == "labels_off"
    if off and candidate == "rt1-own":
        checked = module("rt1_check", folder / "rt1_check.py", EmptyFlows().visit)
        module("rt1_vm", folder / "rt1_vm.py", EmptyFlows().visit, {"EMPTY_FLOW": checked.EMPTY_FLOW})
    if off and candidate == "rt4-pyhost":
        module("library", folder / "library.py", NoExtraLabels().visit)
    adapter = module("lx1_adapter", folder / "adapter.py")
    if not off:
        return adapter
    if candidate == "rt2-py":
        from harness.bindings import neutral_paths
        from subset import Analysis, EMPTY

        class NoLabelAnalysis(Analysis):
            def __init__(self, tree, functions, program, transparent):
                self.tree, self.functions, self.program = tree, functions, program
                self.graph = {name: {n.args[2].id for n in ast.walk(fn)
                                    if isinstance(n, ast.Call) and isinstance(n.func, ast.Name)
                                    and n.func.id == "h_call"} for name, fn in functions.items()}
                self.recursive = {name for name in functions if any(name in self.reachable(child)
                                  for child in self.graph[name])}
                self.dynamic = [n.args[0].value for n in ast.walk(tree) if isinstance(n, ast.Call)
                                and isinstance(n.func, ast.Name) and n.func.id == "h_dynamic"]
                self.marks = {path: (EMPTY, EMPTY) for path in neutral_paths(program)}

        adapter.Analysis = NoLabelAnalysis
    elif candidate == "rt2-node":
        from checker import EMPTY, STATE
        from harness.bindings import neutral_paths

        def effect_plans(program, transparent):
            all_ops, result = neutral_paths(program), {}
            for run in program["runs"]:
                ops = {p: op for p, op in all_ops.items() if p.startswith(run["id"] + "/")}
                pending = [op["module"] for op in ops.values() if op["op"] == "call"]
                seen = set()
                while pending:
                    name = pending.pop()
                    if name in seen:
                        continue
                    seen.add(name)
                    extra = {p: op for p, op in all_ops.items() if p.startswith("modules/" + name + "/")}
                    ops.update(extra)
                    pending += [op["module"] for op in extra.values() if op["op"] == "call"]
                result[run["id"]] = {
                    "knowledge": {p: (EMPTY, EMPTY) for p in ops},
                    "reads": {p for p, op in ops.items() if op["op"] == "kernel" and op["row"] in STATE},
                    "previews": {p for p, op in ops.items() if op["op"] == "preview"},
                    "operations": {op["op"] for op in ops.values()}}
            return result

        adapter.analyze = effect_plans
    elif candidate == "rt3-wasm":
        import frontend
        from harness.common import digest
        from support import read

        class NoLabelAnalysis:
            def __init__(self, surface, transparent):
                self.surface = surface

            def run(self, run_id):
                return {p: (0, 0) for p in frontend.instructions(self.surface)}

        frontend.Analysis = NoLabelAnalysis

        def execute(ctx):
            key = digest(ctx.case["program"])
            surface = adapter.render(ctx.program)
            approved = read(folder / "programs.rt3")[key]
            require(adapter.encode(surface) == adapter.encode(approved), "RT3 local source binding changed")
            compiler = frontend.Compiler(surface, ctx.run["id"], ctx.form, ctx.policy.data["transparent"])
            # Use the existing constant-field path, with all annotations zero.
            # No dynamic metadata imports, joins or static label analysis run.
            compiler.static, compiler.profile = True, 0
            compiler.joined = lambda fn, args, pc, node, output=False: fn.const(0)
            compiled = compiler.compile()
            require(compiled["counts"]["label_or_sites"] == 0, "labels-off emitted a label join")
            require(compiled["counts"]["static_call_sites"].get("meta", 0) == 0,
                    "labels-off emitted a metadata import")
            ctx.local_lowering_sha256 = sha(bytes.fromhex(compiled["wasm_hex"]))
            return adapter.run_compiled(ctx, surface, compiled)

        adapter.execute = execute
    elif candidate == "rt4-pyhost":
        class NoLabelLibrary(adapter.Library):
            def knowledge(self, path, values, *, output=False):
                dependencies = [v.value for v in values]
                return dependencies, set(), any(v.read_state for v in dependencies)

            def make(self, kind, data, path, values=(), *, extra_labels=(), state=False, **metadata):
                dependencies = [v.value for v in values]
                value = self.ctx.host.make(kind, data, path, dependencies, labels=(),
                                           read_state=state or any(v.read_state for v in dependencies), **metadata)
                return self.remember(path, value)

        adapter.Library = NoLabelLibrary
    return adapter
