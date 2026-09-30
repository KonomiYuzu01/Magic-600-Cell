"""Assemble an isolated 0.4 candidate from a current, precompiled native host.

No native build, fixture, GUI, installer or publication is performed here.
"""
import argparse
import ast
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import shutil
import sys
import zipfile

from package_contract import BUNDLE_NAME, EXPERIMENT, VERSION, inspect_payload, sha, write_json

HERE = Path(__file__).resolve().parent
E = HERE.parent
ROOT = E.parents[2]
sys.path.insert(0, str(E))
import build_identity as identity_v2  # noqa: E402  (standard library only)


def constants(path, names):
    values = {}
    for node in ast.parse(path.read_text(encoding='utf-8-sig')).body:
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name) and target.id in names:
                    values[target.id] = ast.literal_eval(node.value)
    if set(values) != set(names):
        raise ValueError('Packaging contract constants missing: ' + str(set(names) - set(values)))
    return values


def allowed_sources():
    contract = constants(E / 'native_launch.py',
        ('RETAINED', 'EXPERIMENT_SOURCES', 'BACKEND_SOURCES', 'SHARED_BACKEND_SOURCES'))
    app = [Path(name) for name in contract['SHARED_BACKEND_SOURCES']]
    app += [EXPERIMENT / name for name in contract['BACKEND_SOURCES']]
    app += [Path('native/directx_runtime.py')]
    for directory in (Path('web'), EXPERIMENT / 'web'):
        app += sorted(p.relative_to(ROOT) for p in (ROOT / directory).iterdir()
                      if p.is_file() and p.suffix in ('.html', '.js', '.css'))
    asset_manifest = json.loads((ROOT / 'assets/manifest.json').read_text(encoding='utf-8-sig'))
    app += [Path('assets/manifest.json')] + [Path(name) for name in asset_manifest['files']]
    app += [Path('native/runtime') / name for name in ('MPUlt.exe', 'MPUlt_puzzles.txt', 'MPUlt_settings.txt')]
    app = sorted(set(app))
    native = [Path('native') / name for name in contract['RETAINED']]
    native += [EXPERIMENT / 'native' / name for name in contract['EXPERIMENT_SOURCES']]
    for relative in app + native:
        if relative.is_absolute() or '..' in relative.parts or not (ROOT / relative).is_file():
            raise ValueError('Missing or unsafe allowlisted source: ' + str(relative))
    check_local_imports(app)
    for name, expected in asset_manifest['files'].items():
        if sha(ROOT / name) != expected:
            raise ValueError('Immutable model asset mismatch: ' + name)
    audit = constants(E / 'endgame_invariants.py', ('AUDIT_SHA256',))['AUDIT_SHA256']
    if sha(E / 'evidence/orbit-invariants-20260916-generators.json') != audit:
        raise ValueError('Pinned endgame invariant report mismatch')
    return contract, app, native, asset_manifest


def check_local_imports(app):
    """Reject a newly imported project module missing from the explicit list."""
    included = set(app)
    for relative in app:
        if relative.suffix != '.py':
            continue
        tree = ast.parse((ROOT / relative).read_text(encoding='utf-8-sig'))
        imports = []
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                imports.extend(alias.name for alias in node.names)
            elif isinstance(node, ast.ImportFrom) and not node.level and node.module:
                imports.append(node.module)
        for module in imports:
            name = Path(*module.split('.')).with_suffix('.py')
            for prefix in (EXPERIMENT, Path('.'), Path('native')):
                dependency = prefix / name
                if (ROOT / dependency).is_file() and dependency not in included:
                    raise ValueError('Local import missing from package allowlist: ' +
                                     str(relative) + ' -> ' + str(dependency))


def checked_native(directory, native, contract):
    directory = directory.resolve()
    build = json.loads((directory / 'build.json').read_text(encoding='utf-8'))
    host = directory / 'Magic600Experiment.exe'
    if not host.is_file() or sha(host) != build['executable_sha256']:
        raise ValueError('Supply the non-regression Magic600Experiment.exe and its matching build.json')
    if build.get('checks', {}).get('after_build', {}).get('status') != 'unchanged':
        raise ValueError('Native build was not bound to unchanged inputs')
    if identity_v2.receipt_version(build) == 2:
        # The recorded after_build status is necessary but never sufficient: the identity must
        # still rehash and every bound product file (runtime included) must match current bytes.
        if not identity_v2.identity_intact(build):
            raise ValueError('Native build receipt does not rehash to its build identity')
        stale = [path for path, digest in identity_v2.product_files(build['identity']['product']).items()
                 if not (ROOT / path).is_file() or sha(ROOT / path) != digest]
        if stale:
            raise ValueError('Native build predates current product inputs: ' + ', '.join(sorted(stale)[:5]))
    expected = {str(p): sha(ROOT / p) for p in native}
    if build.get('source') != expected:
        raise ValueError('Native host does not match all current retained and experimental C# sources')
    backend = [EXPERIMENT / name for name in contract['BACKEND_SOURCES']]
    backend += [Path(name) for name in contract['SHARED_BACKEND_SOURCES']]
    if build.get('backend_sources') != {str(p): sha(ROOT / p) for p in backend}:
        raise ValueError('Native build receipt predates the current backend; request a current bound build')
    if sha(host.with_suffix('.exe.config')) != sha(ROOT / 'native/NativeHost.exe.config'):
        raise ValueError('Native CLR configuration differs from the approved retained host')
    return host, build


def copy_file(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def validate_assembly_inputs(stage, source_hashes, app_files, native_files, contract, host, native_build):
    if any(sha(ROOT / name) != expected for name, expected in source_hashes.items()):
        raise ValueError('Sources changed during assembly; this output is not accepted')
    if sha(host) != native_build['executable_sha256']:
        raise ValueError('Native host changed during assembly')
    # The first native check precedes source capture. Recheck their relationship,
    # not just each file's stability, so an edit in that interval cannot pass.
    _, current_build = checked_native(host.parent, native_files, contract)
    if current_build != native_build:
        raise ValueError('Native build receipt changed during assembly')
    staged = {stage / 'app' / p: source_hashes[p.as_posix()] for p in app_files}
    staged[stage / 'native/Magic600Native.exe'] = native_build['executable_sha256']
    staged[stage / 'native/Magic600Native.exe.config'] = source_hashes['native/NativeHost.exe.config']
    for path, expected in staged.items():
        if not path.is_file() or sha(path) != expected:
            raise ValueError('Staged file differs from its bound input: ' + str(path.relative_to(stage)))


def copy_licenses(bundle):
    files = {'LICENSE': 'LICENSE', 'CREDITS.md': 'CREDITS.md',
             'THIRD_PARTY_NOTICES.md': 'licenses/THIRD_PARTY_NOTICES.md',
             'native/LICENSE.MPUlt.txt': 'licenses/MPUlt-MIT.txt'}
    for source, destination in files.items():
        copy_file(ROOT / source, bundle / destination)
    copy_file(Path(sys.base_prefix) / 'LICENSE.txt', bundle / 'licenses/Python-LICENSE.txt')
    for name, label in (('numpy', 'NumPy'), ('pyinstaller', 'PyInstaller'),
                        ('charset-normalizer', 'Charset-Normalizer'), ('typing_extensions', 'Typing-Extensions')):
        distribution = importlib.metadata.distribution(name)
        records = [record for record in distribution.files or ()
                   if 'licenses' in record.parts or record.name.lower().startswith(('license', 'licence', 'copying', 'notice'))]
        if not records:
            raise ValueError('Missing complete license records for ' + name)
        for record in records:
            if 'licenses' in record.parts:
                relative = Path(*record.parts[record.parts.index('licenses') + 1:])
            elif any(part.endswith('.dist-info') for part in record.parts):
                index = next(i for i, part in enumerate(record.parts) if part.endswith('.dist-info'))
                relative = Path(*record.parts[index + 1:])
            else:
                relative = Path(*record.parts)
            if relative.is_absolute() or '..' in relative.parts:
                raise ValueError('Unsafe license path: ' + str(record))
            source = Path(distribution.locate_file(record))
            if source.is_file():
                copy_file(source, bundle / 'licenses' / label / relative)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native-build', type=Path, required=True, help='Current product host directory from native_launch.build; never a regression EXE.')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--work', type=Path, required=True)
    parser.add_argument('--toolchain', type=Path, default=HERE / 'toolchain')
    parser.add_argument('--no-zip', action='store_true')
    args = parser.parse_args()
    if os.name != 'nt' or sys.maxsize <= 2**32:
        raise ValueError('Use 64-bit CPython on Windows')
    sys.path.insert(0, str(args.toolchain.resolve()))
    import PyInstaller
    from PyInstaller.__main__ import run as freeze
    if PyInstaller.__version__ != '6.22.2':
        raise ValueError('Use the pinned isolated requirements-build.txt toolchain')
    contract, app_files, native_files, asset_manifest = allowed_sources()
    host, native_build = checked_native(args.native_build, native_files, contract)
    bundle = args.output.resolve() / BUNDLE_NAME
    work = args.work.resolve()
    if bundle.exists() or work.exists():
        raise ValueError('Choose fresh bundle and work directories; no prior output is overwritten')
    work.mkdir(parents=True)
    bundle.parent.mkdir(parents=True, exist_ok=True)
    source_files = app_files + native_files + [Path('native/NativeHost.exe.config'),
        Path('LICENSE'), Path('CREDITS.md'), Path('THIRD_PARTY_NOTICES.md'), Path('native/LICENSE.MPUlt.txt')]
    source_files += [p.relative_to(ROOT) for p in HERE.iterdir() if p.is_file() and p.suffix in ('.py', '.spec', '.txt', '.md')]
    source_hashes = {p.as_posix(): sha(ROOT / p) for p in sorted(set(source_files))}
    stage = work / 'stage'
    for relative in app_files:
        copy_file(ROOT / relative, stage / 'app' / relative)
    copy_file(host, stage / 'native/Magic600Native.exe')
    copy_file(host.with_suffix('.exe.config'), stage / 'native/Magic600Native.exe.config')
    write_json(stage / 'freeze.json', dict(paths=[str(HERE), str(ROOT), str(ROOT / 'native'), str(E)],
        modules=sorted({p.stem for p in app_files if p.suffix == '.py'})))
    (stage / 'version.txt').write_text("VSVersionInfo(ffi=FixedFileInfo(filevers=(0,4,0,0),prodvers=(0,4,0,0),mask=0x3f,flags=0,OS=0x40004,fileType=1,subtype=0,date=(0,0)),kids=[StringFileInfo([StringTable('040904B0',[StringStruct('FileDescription','Magic 600 Cell native workspace'),StringStruct('FileVersion','0.4'),StringStruct('ProductName','Magic 600 Cell'),StringStruct('ProductVersion','0.4')])]),VarFileInfo([VarStruct('Translation',[1033,1200])])])", encoding='utf-8')
    os.environ['MAGIC600_PACKAGE_STAGE'] = str(stage)
    # Pin the build subprocess search path to this local toolchain only.
    os.environ['PYTHONPATH'] = str(args.toolchain.resolve())
    freeze(['--noconfirm', '--clean', '--distpath', str(bundle.parent),
            '--workpath', str(work / 'pyinstaller'), str(HERE / 'Magic600Cell.spec')])
    copy_licenses(bundle)
    for name in ('README.md', 'CHANGES.md'):
        copy_file(HERE / name, bundle / name)
    files = inspect_payload(bundle)
    validate_assembly_inputs(stage, source_hashes, app_files, native_files, contract, host, native_build)
    entries = [dict(path=p.relative_to(bundle).as_posix(), sha256=sha(p), bytes=p.stat().st_size)
               for p in sorted(files)]
    manifest = dict(version=VERSION, layout_version=1,
        stage='Isolated final-validation candidate; not a public release or final acceptance',
        layout='x64 frozen launcher/engine; unchanged source-layout backend; x86 retained native host',
        model_id=asset_manifest['model_id'], source_files=source_hashes,
        app_allowlist=[p.as_posix() for p in app_files],
        native=dict(build_identity=native_build['build_identity'], executable_sha256=sha(host),
                    sources=native_build['source']),
        toolchain=dict(python=sys.version, numpy=importlib.metadata.version('numpy'),
                       charset_normalizer=importlib.metadata.version('charset-normalizer'),
                       typing_extensions=importlib.metadata.version('typing_extensions'),
                       pyinstaller=PyInstaller.__version__),
        pinned_invariants_sha256=sha(E / 'evidence/orbit-invariants-20260916-generators.json'),
        normal_startup_fixture_run=False, microsoft_managed_directx_included=False,
        files=entries)
    write_json(bundle / '_internal/package-manifest.json', manifest)
    report = dict(assembled=True, final_acceptance=False, bundle=str(bundle), files=len(entries),
        bytes=sum(row['bytes'] for row in entries), inputs_unchanged=True,
        native_sha256=sha(host), manifest_sha256=sha(bundle / '_internal/package-manifest.json'))
    if not args.no_zip:
        archive = bundle.parent / (BUNDLE_NAME + '.zip')
        if archive.exists():
            raise ValueError('Existing archive is preserved; choose another output directory')
        with zipfile.ZipFile(archive, 'x', zipfile.ZIP_DEFLATED, compresslevel=6) as target:
            for path in sorted(bundle.rglob('*')):
                if path.is_file():
                    target.write(path, path.relative_to(bundle.parent))
        report.update(archive=str(archive), archive_sha256=sha(archive))
    write_json(work / 'assembly-report.json', report)
    print(json.dumps(report), flush=True)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
