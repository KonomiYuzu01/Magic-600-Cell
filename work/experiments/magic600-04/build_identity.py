"""Build identity v2 for the retained native host (0.4.1 step 1).

The identity is the SHA-256 of canonical JSON over one immutable payload,
stored in a receipt under the key "identity":

- product: every file the shipped program is built from or starts with:
  C# and Python sources, the CLR configuration, the model, and the retained
  runtime (MPUlt.exe, its puzzle definitions and distributed default settings,
  and the DirectX loader). Keys are POSIX paths relative to the repository root.
- compile_recipe: the flags and reference names of the sealed csc invocation
  (native/bootstrap.py compile_recipe); the compiler is a role, not a path.
- compiler: the csc banner and, by role, the compiler files and every
  reference assembly the recipe names.
- interpreter: implementation, exact version and bits; the base interpreter
  binaries and the environment launcher by role; the NumPy version and a
  digest of its immutable wheel payload (see numpy_payload).

Outside the identity, and free to change without changing it: the checkout
location, the operating-system build, harness files (tests, fixtures and the
non-recipe parts of native_launch.py and native/bootstrap.py), user-modified
settings and launch evidence. The standard library is covered by the exact
interpreter version and base binaries, not file by file. The executable is not
bit-reproducible: the legacy compiler has no deterministic mode, so a build is
reused only when its recorded executable hash still matches.
"""
from __future__ import annotations

import base64
import csv
import hashlib
import json
import os
import platform
import struct
import sys
from pathlib import Path

EVIDENCE_VERSION = 2
RUNTIME_FILES = ('native/runtime/MPUlt.exe', 'native/runtime/MPUlt_puzzles.txt',
                 'native/runtime/MPUlt_settings.txt', 'native/directx_runtime.py')
# Every top-level key the 0.4 launcher and harness wrote, including after a launch (run_outcome)
# and in harness receipts (scope, executable).
V1_SECTIONS = {'evidence_version', 'sources', 'model', 'toolchain', 'checks', 'build_identity', 'artifacts',
               'executable_sha256', 'source', 'backend_sources', 'backend_sha256', 'stage', 'approvals', 'runtime',
               'run_outcome', 'scope', 'executable'}
V2_REQUIRED = {'evidence_version', 'identity', 'build_identity', 'artifacts', 'executable_sha256', 'checks',
               'source', 'backend_sources', 'harness'}
IDENTITY_KEYS = {'product', 'compile_recipe', 'compiler', 'interpreter'}
# Written by the installer, not taken from the wheel: excluded from the NumPy payload.
GENERATED_DIST_FILES = {'RECORD', 'INSTALLER', 'REQUESTED', 'direct_url.json', 'uv_cache.json'}
COMPILER_FILES = ('csc.exe', 'csc.exe.config', 'alink.dll', '1033/cscui.dll', '1033/alinkui.dll',
                  'default.win32manifest')


def sha256_file(path) -> str:
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def canonical(value) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=True).encode()


def digest(value) -> str:
    return hashlib.sha256(canonical(value)).hexdigest()


def relative(path, root) -> str:
    """POSIX path of a file inside root; refuses anything outside it."""
    rel = os.path.relpath(Path(path).resolve(), Path(root).resolve())
    if rel == '..' or rel.startswith('..' + os.sep) or os.path.isabs(rel):
        raise ValueError('Input outside the repository: ' + str(path))
    return rel.replace(os.sep, '/')


def hash_inputs(paths, root) -> dict:
    return {relative(p, root): sha256_file(p) for p in dict.fromkeys(paths)}


def model_binding(root) -> dict:
    root = Path(root)
    manifest_path = root / 'assets/manifest.json'
    manifest = json.loads(manifest_path.read_text(encoding='utf-8-sig'))
    assets = {name.replace('\\', '/'): sha256_file(root / name) for name in manifest['files']}
    for name, expected in manifest['files'].items():
        if assets[name.replace('\\', '/')] != expected:
            raise ValueError('Asset hash mismatch: ' + name)
    return dict(model_id=manifest['model_id'], manifest=hash_inputs([manifest_path], root), assets=assets)


def product_binding(sources, root) -> dict:
    """sources: every C#, Python and configuration file of the product build."""
    root = Path(root)
    return dict(sources=hash_inputs(sources, root), model=model_binding(root),
                runtime=hash_inputs([root / name for name in RUNTIME_FILES], root))


def product_files(product) -> dict:
    """Every product file with its expected hash, for rechecking."""
    files = dict(product['sources'])
    files.update(product['model']['manifest'])
    files.update(product['model']['assets'])
    files.update(product['runtime'])
    return files


def compiler_binding(csc, recipe, banner) -> dict:
    """Role hashes of the compiler files and of every reference assembly in the recipe."""
    framework = Path(csc).resolve().parent
    files = {}
    for name in COMPILER_FILES:
        path = framework / name
        files['compiler:' + name] = sha256_file(path) if path.is_file() else None
    for name in recipe['references']:
        path = framework / name
        files['reference:' + name.lower()] = sha256_file(path) if path.is_file() else None
    missing = sorted(role for role, value in files.items() if value is None)
    if missing:
        raise ValueError('Compiler files missing: ' + ', '.join(missing))
    return dict(banner=banner, files=files)


def _record_hash(value: str) -> str | None:
    algorithm, _, encoded = value.partition('=')
    if algorithm != 'sha256' or not encoded:
        return None
    return base64.urlsafe_b64decode(encoded + '=' * (-len(encoded) % 4)).hex()


def numpy_payload(site_packages, dist_info) -> dict:
    """Digest of NumPy's immutable wheel payload, verified against its RECORD.

    Included: every RECORD entry inside site-packages except the generated files
    installers write (RECORD itself, INSTALLER, REQUESTED, direct_url.json,
    uv_cache.json) and bytecode. Excluded as well: console-script wrappers, which
    RECORD lists outside site-packages ("../") and which embed their install path.
    Any other entry without a sha256 is refused, as is any file whose bytes
    differ from RECORD."""
    site_packages = Path(site_packages)
    dist = site_packages / dist_info
    entries = {}
    with (dist / 'RECORD').open(encoding='utf-8', newline='') as stream:
        for row in csv.reader(stream):
            if not row:
                continue
            name = row[0].replace('\\', '/')
            if name.startswith('../') or name.startswith('/') or '__pycache__/' in name or name.endswith('.pyc'):
                continue
            if name.startswith(dist_info + '/') and name.split('/', 1)[1] in GENERATED_DIST_FILES:
                continue
            expected = _record_hash(row[1]) if len(row) > 1 else None
            if expected is None:
                raise ValueError('NumPy RECORD entry without a sha256: ' + name)
            actual = sha256_file(site_packages / name)
            if actual != expected:
                raise ValueError('NumPy file differs from its RECORD: ' + name)
            entries[name] = actual
    return dict(files=len(entries), digest=digest(entries))


def interpreter_binding(implementation, version, bits, base_prefix, launcher, numpy_version, numpy) -> dict:
    base = Path(base_prefix)
    major, minor = version.split('.')[:2]
    files = {}
    for role, path in (('base:python.exe', base / 'python.exe'), ('base:python3.dll', base / 'python3.dll'),
                       (f'base:python{major}{minor}.dll', base / f'python{major}{minor}.dll'),
                       ('launcher', Path(launcher))):
        files[role] = sha256_file(path) if path.is_file() else None
    missing = sorted(role for role, value in files.items() if value is None)
    if missing:
        raise ValueError('Interpreter files missing: ' + ', '.join(missing))
    return dict(implementation=implementation, version=version, bits=bits, files=files,
                numpy=dict(version=numpy_version, payload=numpy))


def current_interpreter() -> dict:
    """The binding of the interpreter running this process (the pinned engine environment)."""
    import numpy
    from importlib.metadata import distribution
    dist = distribution('numpy')
    dist_dir = Path(dist._path)  # the installed .dist-info directory
    # Bind only the NumPy actually imported: a shadow earlier on sys.path must not
    # inherit the identity of the untouched installed distribution.
    imported = Path(getattr(numpy, '__file__', None) or '').resolve()
    if imported != (dist_dir.parent / 'numpy/__init__.py').resolve() or numpy.__version__ != dist.version:
        raise ValueError('Imported NumPy is not the installed distribution')
    payload = numpy_payload(dist_dir.parent, dist_dir.name)
    return interpreter_binding(sys.implementation.name, platform.python_version(), struct.calcsize('P') * 8,
                               sys.base_prefix, sys.executable, numpy.__version__, payload)


def identity_payload(product, recipe, compiler, interpreter) -> dict:
    return dict(product=product, compile_recipe=recipe, compiler=compiler, interpreter=interpreter)


def receipt_version(receipt) -> int:
    """1 or 2, only when the receipt's sections match that version; anything else is refused."""
    version = receipt.get('evidence_version') if isinstance(receipt, dict) else None
    if type(version) is not int or version not in (1, 2):
        raise ValueError('Unsupported or missing evidence_version')
    keys = set(receipt)
    if version == 1:
        if 'identity' in keys or 'harness' in keys or not {'sources', 'model', 'toolchain', 'build_identity'} <= keys \
                or not keys <= V1_SECTIONS:
            raise ValueError('Receipt labelled evidence_version 1 does not have the v1 sections')
        return 1
    missing = V2_REQUIRED - keys
    if missing or 'toolchain' in keys or 'sources' in keys:
        raise ValueError('Receipt labelled evidence_version 2 does not have the v2 sections: ' + ', '.join(sorted(missing)))
    if not isinstance(receipt['identity'], dict) or set(receipt['identity']) != IDENTITY_KEYS:
        raise ValueError('Receipt identity does not have exactly ' + ', '.join(sorted(IDENTITY_KEYS)))
    return 2


def identity_intact(receipt) -> bool:
    return digest(receipt['identity']) == receipt.get('build_identity')


def expected_product_paths(root, sources) -> dict:
    """The product inventory that code, not a receipt, declares: the product sources given,
    the model manifest and every asset it lists, and the retained runtime files."""
    root = Path(root)
    manifest = json.loads((root / 'assets/manifest.json').read_text(encoding='utf-8-sig'))
    return dict(sources={relative(p, root) for p in sources}, manifest={'assets/manifest.json'},
                assets={name.replace('\\', '/') for name in manifest['files']}, runtime=set(RUNTIME_FILES))


def _is_sha256(value) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(c in '0123456789abcdef' for c in value)


def inventory_problems(identity, expected, recipe) -> list:
    """Why a v2 identity does not bind the complete expected inputs; empty when it does.

    expected: expected_product_paths(); recipe: the recipe the current code compiles with.
    A receipt that drops a file or a tool role, or declares extra ones, is refused even
    when its identity rehashes, because the digest only proves what the receipt declares."""
    problems = []
    product = identity['product']
    declared = dict(sources=product.get('sources'), manifest=product.get('model', {}).get('manifest'),
                    assets=product.get('model', {}).get('assets'), runtime=product.get('runtime'))
    for section, paths in expected.items():
        files = declared[section]
        if not isinstance(files, dict):
            problems.append('product ' + section + ' missing')
            continue
        for path in sorted(paths - set(files)):
            problems.append('product ' + section + ' lacks ' + path)
        for path in sorted(set(files) - paths):
            problems.append('product ' + section + ' has undeclared ' + path)
        problems.extend('product ' + section + ' has no sha256 for ' + path
                        for path, value in sorted(files.items()) if not _is_sha256(value))
    if identity['compile_recipe'] != recipe:
        problems.append('compile recipe differs from the current recipe')
    roles = {'compiler:' + name for name in COMPILER_FILES}
    roles |= {'reference:' + name.lower() for name in recipe['references']}
    problems.extend(_role_problems('compiler', identity['compiler'], roles))
    interpreter = identity['interpreter']
    version = str(interpreter.get('version', ''))
    major_minor = ''.join(version.split('.')[:2])
    roles = {'base:python.exe', 'base:python3.dll', 'base:python' + major_minor + '.dll', 'launcher'}
    problems.extend(_role_problems('interpreter', interpreter, roles))
    numpy = interpreter.get('numpy')
    if not (isinstance(numpy, dict) and numpy.get('version') and isinstance(numpy.get('payload'), dict)
            and _is_sha256(numpy['payload'].get('digest')) and type(numpy['payload'].get('files')) is int
            and numpy['payload']['files'] > 0):
        problems.append('interpreter numpy binding incomplete')
    return problems


def _role_problems(section, binding, roles) -> list:
    files = binding.get('files') if isinstance(binding, dict) else None
    if not isinstance(files, dict):
        return [section + ' files missing']
    problems = [section + ' lacks role ' + role for role in sorted(roles - set(files))]
    problems += [section + ' has undeclared role ' + role for role in sorted(set(files) - roles)]
    problems += [section + ' has no sha256 for ' + role for role, value in sorted(files.items()) if not _is_sha256(value)]
    return problems


def legacy_aliases(product, root, retained_and_experiment, backend) -> dict:
    """The v1 'source'/'backend_sources' keys (str(Path) relative to root) that packaging still reads."""
    root = Path(root)
    by_posix = product['sources']
    alias = lambda paths: {str(Path(p).relative_to(root)): by_posix[relative(p, root)] for p in paths}
    return dict(source=alias(retained_and_experiment), backend_sources=alias(backend))
