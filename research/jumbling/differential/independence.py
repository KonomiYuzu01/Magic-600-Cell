"""Static import policies for the harness and the independently built candidate."""

import ast
from dataclasses import dataclass
from pathlib import Path
import sys


_ROOT = Path(__file__).resolve().parent
_DYNAMIC_NAMES = frozenset(('__import__', 'importlib', 'runpy', 'exec', 'eval', 'compile'))


def harness_files() -> list[Path]:
    return sorted(_ROOT.glob('*.py'))


def candidate_files() -> list[Path]:
    return sorted((_ROOT / 'candidate').rglob('*.py'))


@dataclass(frozen=True)
class ImportPolicy:
    name: str
    package: str

    def allows(self, module):
        top = module.split('.')[0]
        if top in sys.stdlib_module_names or top == 'numpy':
            return True
        if self.name == 'candidate':
            return module == 'differential.candidate' or module.startswith('differential.candidate.')
        if module in ('sim', 'exact') or module.startswith('sim.'):
            return True
        modules = {p.stem for p in harness_files() if p.stem != '__init__'}
        return module == 'differential' or module in {'differential.' + name for name in modules}


HARNESS_POLICY = ImportPolicy('harness', 'differential')
CANDIDATE_POLICY = ImportPolicy('candidate', 'differential.candidate')


@dataclass(frozen=True)
class Violation:
    path: str
    line: int
    message: str


def _package(path, policy):
    parts = Path(path).parts
    if 'differential' in parts:
        start = len(parts) - 1 - parts[::-1].index('differential')
        return '.'.join(parts[start:-1])
    return policy.package


def _resolve(node, package):
    if not node.level:
        return node.module or ''
    parts = package.split('.')
    if node.level > len(parts):
        return None
    base = parts[:len(parts) - node.level + 1]
    return '.'.join(base + ([node.module] if node.module else []))


def scan_source(source, policy, *, path='<string>') -> list[Violation]:
    """Scan syntax only. Relative imports use the source path or policy package."""
    if policy not in (HARNESS_POLICY, CANDIDATE_POLICY):
        raise ValueError('unknown independence policy')
    path = str(path)
    try:
        tree = ast.parse(source, filename=path)
    except SyntaxError as exc:
        return [Violation(path, exc.lineno or 1, 'invalid Python syntax: ' + exc.msg)]
    violations = []
    sys_names = {'sys'}
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            sys_names.update(a.asname or 'sys' for a in node.names if a.name == 'sys')

    def flag(node, message):
        violations.append(Violation(path, node.lineno, message))

    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                if not policy.allows(alias.name):
                    flag(node, 'forbidden import: ' + alias.name)
                if alias.name.split('.')[0] in _DYNAMIC_NAMES or (alias.asname or '') in _DYNAMIC_NAMES:
                    flag(node, 'dynamic code name in import: ' + alias.name)
        elif isinstance(node, ast.ImportFrom):
            module = _resolve(node, _package(path, policy))
            if module is None:
                flag(node, 'forbidden relative import beyond the package')
            elif module == 'differential':
                for alias in node.names:
                    imported = module + '.' + alias.name
                    if not policy.allows(imported):
                        flag(node, 'forbidden import: ' + imported)
            elif not policy.allows(module):
                flag(node, 'forbidden import: ' + module)
            if module and module.split('.')[0] in _DYNAMIC_NAMES:
                flag(node, 'dynamic code module: ' + module)
            for alias in node.names:
                if alias.name in _DYNAMIC_NAMES or (alias.asname or '') in _DYNAMIC_NAMES:
                    flag(node, 'dynamic code name in import: ' + alias.name)
                if module == 'sys' and alias.name == 'path':
                    flag(node, 'reference to sys.path')
        elif isinstance(node, ast.Name) and node.id in _DYNAMIC_NAMES:
            flag(node, 'dynamic code name: ' + node.id)
        elif (isinstance(node, ast.Attribute) and node.attr == 'path'
              and isinstance(node.value, ast.Name) and node.value.id in sys_names):
            flag(node, 'reference to sys.path')
    return sorted(set(violations), key=lambda v: (v.line, v.message))


def scan_paths(paths, policy) -> list[Violation]:
    violations = []
    for path in sorted(map(Path, paths)):
        violations.extend(scan_source(path.read_text(encoding='utf-8'), policy, path=str(path)))
    return violations
