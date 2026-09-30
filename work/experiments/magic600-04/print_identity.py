"""Print checkout and build identities without compiling or writing files."""
import sys

sys.dont_write_bytecode = True

import ast
import json

from native_launch import (ROOT, HERE, HARNESS_SOURCES, compile_recipe, current_tools,
                           identity_v2, native_sources, product_sources)


def harness_inputs():
    runner = HERE / 'tests/run_postapproval.py'
    tree = ast.parse(runner.read_text(encoding='utf-8'))
    main = next((node for node in tree.body
                 if isinstance(node, ast.FunctionDef) and node.name == 'main'), None)
    names = [node.value for node in ast.walk(main)
             if isinstance(node, ast.Constant) and isinstance(node.value, str)
             and node.value.endswith('.cs')] if main is not None else []
    if len(names) != 22:
        raise ValueError('Expected 22 C# harness checks')
    return (list(HARNESS_SOURCES) + [HERE / 'tests' / name for name in names]
            + [HERE / 'tests/final_workflow_cases.json', runner])


def report():
    harness = identity_v2.hash_inputs(harness_inputs(), ROOT)
    product = identity_v2.product_binding(product_sources(native_sources()), ROOT)
    recipe = compile_recipe('ExperimentProgram')
    tools = current_tools({'compile_recipe': recipe})
    unavailable = [role for role in ('compiler', 'interpreter') if tools[role] is None]
    build_identity = None
    if not unavailable:
        build_identity = identity_v2.digest(identity_v2.identity_payload(
            product, recipe, tools['compiler'], tools['interpreter']))
    return dict(format='magic600-identity-print-v1',
                source_identity=identity_v2.digest({'product': product, 'compile_recipe': recipe}),
                product_files=len(identity_v2.product_files(product)),
                harness=harness, build_identity=build_identity,
                build_identity_unavailable=unavailable,
                notes='source_identity is not build_identity; it excludes compiler and interpreter bindings.')


def main():
    if sys.argv[1:]:
        print('usage: python work/experiments/magic600-04/print_identity.py', file=sys.stderr)
        return 2
    try:
        data = report()
    except (OSError, ValueError, SyntaxError):
        # Upstream exceptions can contain private paths and multiline diagnostics.
        print('Identity inputs are missing or invalid.', file=sys.stderr)
        return 1
    print(json.dumps(data, sort_keys=True, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
