"""Build and run isolated native review samples using the retained Windows runtime."""
from pathlib import Path
import argparse
import hashlib
import json
import os
import platform
import shutil
import struct
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT))
from native.bootstrap import compile_program, compile_recipe
from native.directx_runtime import find_directx
from native.inspect_runtime import inspect
from engine_process import EngineProcess
sys.path.insert(0, str(HERE))
import build_identity as identity_v2

RETAINED = ('NativeHost.cs', 'NativeDockLayout.cs', 'NativeDiagnostics.cs',
    'NativeRendererLifecycle.cs', 'NativeSnapshot.cs', 'NativeStickerAccess.cs',
    'NativeRenderSubset.cs', 'NativePickingVisibility.cs', 'NativeFullRenderer.cs',
    'NativeColorGraph.cs', 'NativeStructureExplorer.cs', 'NativeAuxiliaryViews.cs')
EXPERIMENT_SOURCES = ('ExperimentCellView.cs', 'ExperimentInput.cs', 'ExperimentHub.cs',
    'ExperimentBridge.cs', 'ExperimentShell.cs', 'ExperimentWorkspace.cs', 'ExperimentTools.cs',
    'ExperimentKeyboard.cs', 'ExperimentPhase.cs', 'ExperimentFilter.cs', 'ExperimentWindows.cs', 'ExperimentWorkSheets.cs', 'ExperimentWindowPlacement.cs', 'ExperimentMacroLibrary.cs', 'ExperimentHelp.cs', 'ExperimentMacroRelations.cs', 'ExperimentSolveWindow.cs', 'ExperimentCycles.cs', 'ExperimentHubCycles.cs', 'ExperimentRecommendation.cs', 'ExperimentReferenceVariants.cs', 'ExperimentEndgame.cs', 'ExperimentResidualWorkflow.cs', 'ExperimentCandidates.cs', 'ExperimentSession.cs', 'ExperimentDisplay.cs')
BACKEND_SOURCES = ('engine.py', 'adapter.py', 'draft_inspection.py', 'filter_projection.py',
    'grip_frames.py', 'keymap_catalog.py', 'mathematical_names.py', 'transported_frames.py', 'residuals.py', 'work_intents.py', 'current_recommendation.py', 'position_requirements.py', 'local_geometry.py', 'work_sheets.py', 'macro_library.py', 'macro_use.py', 'cycle_projection.py', 'reference_variants.py', 'endgame_library.py', 'endgame_invariants.py', 'orbit_invariants.py', 'workflow_continuity.py', 'candidate_analysis.py', 'session_workflow.py', 'evidence/orbit-invariants-20260916-generators.json')
SHARED_BACKEND_SOURCES = ('grips.py', 'server.py', 'core.py', 'session.py',
    'session_lock.py', 'enhanced.py', 'native_bridge.py', 'engine_process.py',
    'log_io.py', 'mpult_log.py')
# Harness files: bound in every receipt but outside the build identity, so editing
# them never changes it. The recipe these scripts produce is bound separately.
HARNESS_SOURCES = (HERE / 'native_launch.py', ROOT / 'native/bootstrap.py', ROOT / 'native/inspect_runtime.py')
PRODUCT_CONFIG = ROOT / 'native/NativeHost.exe.config'
COMPILER = Path(os.environ.get('WINDIR', r'C:\Windows')) / 'Microsoft.NET/Framework/v4.0.30319/csc.exe'


def hash_file(path):
    return identity_v2.sha256_file(path)


def hash_files(paths, root=None):
    """POSIX repository paths and hashes; a path outside the repository is refused, never serialized."""
    return identity_v2.hash_inputs(paths, ROOT if root is None else root)


def product_sources(sources):
    return (list(sources) + [HERE / name for name in BACKEND_SOURCES]
            + [ROOT / name for name in SHARED_BACKEND_SOURCES] + [PRODUCT_CONFIG])


def native_sources():
    return [ROOT / 'native' / name for name in RETAINED] + [HERE / 'native' / name for name in EXPERIMENT_SOURCES]


def product_inventory():
    """What a product receipt must bind, declared by this code rather than by the receipt."""
    return (identity_v2.expected_product_paths(ROOT, product_sources(native_sources())),
            compile_recipe('ExperimentProgram'))


def compiler_banner(compiler):
    info = subprocess.run([str(compiler), '/help'], stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        encoding='utf-8', errors='replace', timeout=20, check=True)
    return info.stdout.splitlines()[0]


def current_tools(identity):
    """Re-resolve the compiler and interpreter now; recorded tool paths are never trusted or stored."""
    tools = {}
    try:
        tools['compiler'] = identity_v2.compiler_binding(COMPILER, identity['compile_recipe'], compiler_banner(COMPILER))
    except (OSError, ValueError, subprocess.SubprocessError):
        tools['compiler'] = None
    try:
        tools['interpreter'] = identity_v2.current_interpreter()
    except (OSError, ValueError, ImportError):
        tools['interpreter'] = None
    return tools


def capture_identity(sources, main_type, harness=()):
    """A v2 receipt for a build of `sources` with compile_recipe(main_type)."""
    recipe = compile_recipe(main_type)
    identity = identity_v2.identity_payload(
        product=identity_v2.product_binding(product_sources(sources), ROOT), recipe=recipe,
        compiler=identity_v2.compiler_binding(COMPILER, recipe, compiler_banner(COMPILER)),
        interpreter=identity_v2.current_interpreter())
    return dict(evidence_version=identity_v2.EVIDENCE_VERSION, identity=identity,
        build_identity=identity_v2.digest(identity),
        harness=identity_v2.hash_inputs(list(HARNESS_SOURCES) + list(harness), ROOT),
        environment=dict(platform=platform.platform()), checks={})


def _check_tools(identity, current, changed, missing):
    """Compare recorded tool facts and role hashes with the re-resolved tools."""
    for section in ('compiler', 'interpreter'):
        recorded, now = identity[section], current.get(section)
        if now is None:
            missing.append(section)
            continue
        for key in sorted(set(recorded) - {'files'}):
            if recorded[key] != now.get(key):
                changed.append(section + ':' + key)
        for role, checksum in sorted(recorded['files'].items()):
            if role not in now['files']:
                missing.append(section + ':' + role)
            elif now['files'][role] != checksum:
                changed.append(section + ':' + role)
    return len(identity['compiler']['files']) + len(identity['interpreter']['files'])


def check_evidence(evidence, phase, root=ROOT, tools=current_tools, inventory=product_inventory, private=()):
    """Hash only declared immutable inputs; record settings changes separately.

    A v2 receipt must also rehash to its build identity and bind the complete inventory this
    code declares, and its compiler and interpreter roles are re-resolved and compared; a
    missing role counts as missing. Bindings from different sections are never merged: two
    different hashes for one path are a conflict. `private` holds (label, path, sha256)
    for local tools whose path is checked but never written to the receipt."""
    version = identity_v2.receipt_version(evidence)
    runtime = evidence.get('runtime', {})
    changed, missing = [], []
    if version == 1:
        model = evidence.get('model', {})
        groups = [evidence.get('sources', {}), model.get('manifest', {}), model.get('assets', {}),
            evidence.get('toolchain', {}).get('files', {})]
        tool_count = 0
    else:
        if not identity_v2.identity_intact(evidence):
            changed.append('identity')
        expected_paths, recipe = inventory()
        changed.extend('inventory: ' + problem for problem in
                       identity_v2.inventory_problems(evidence['identity'], expected_paths, recipe))
        groups = [identity_v2.product_files(evidence['identity']['product']), evidence.get('harness', {})]
        tool_count = _check_tools(evidence['identity'], tools(evidence['identity']), changed, missing)
    groups += [evidence.get('artifacts', {}), runtime.get('origins', {}), runtime.get('external_origins', {}),
               runtime.get('immutable', {})]
    expected = {}
    for group in groups:
        for path, checksum in group.items():
            expected.setdefault(os.path.normcase(os.path.normpath(path)), (path, set()))[1].add(checksum)
    for path, checksums in expected.values():
        if len(checksums) > 1:
            changed.append(path + ' (conflicting bindings)')
            continue
        try:
            if hash_file(root / path) != next(iter(checksums)):
                changed.append(path)
        except OSError:
            missing.append(path)
    for label, path, checksum in private:
        try:
            if hash_file(path) != checksum:
                changed.append(label)
        except OSError:
            missing.append(label)
    evidence.setdefault('checks', {})[phase] = dict(
        status='changed' if changed or missing else 'unchanged',
        checked_files=len(expected) + tool_count + len(private), changed=changed, missing=missing)
    if runtime.get('mutable_initial'):
        runtime['mutable_' + phase] = {
            name: hash_file(root / name) if (root / name).is_file() else None
            for name in runtime['mutable_initial']}
    return not changed and not missing


def write_evidence(path, evidence):
    path.write_text(json.dumps(evidence, indent=2), encoding='utf-8')


def prepare_runtime(runtime):
    """Keep the supported loader and bind the actual copied assemblies."""
    source = ROOT / 'native/runtime/MPUlt.exe'
    metadata = inspect(source)
    if metadata['missing']:
        raise RuntimeError('Retained native reflection contract failed: ' + ', '.join(metadata['missing']))
    runtime.mkdir(parents=True, exist_ok=True)
    assemblies = {'MPUlt.exe': source, **find_directx(preferred=(source.parent, runtime))}
    origins, external = {}, {}
    for name, path in assemblies.items():
        checksum = hash_file(path)
        if path.resolve() != (runtime / name).resolve():
            shutil.copy2(path, runtime / name)
        if hash_file(runtime / name) != checksum:
            raise RuntimeError('Runtime copy changed: ' + name)
        try:
            origins[identity_v2.relative(path, ROOT)] = checksum
        except ValueError:
            # A system or user installation: bind its copy to the origin's hash; its path is never stored.
            external[identity_v2.relative(runtime / name, ROOT)] = checksum
    for name in ('MPUlt_puzzles.txt', 'MPUlt_settings.txt'):
        if not (runtime / name).exists():
            shutil.copy2(source.parent / name, runtime / name)
    # No supported fixture writes puzzle definitions. Only preferences are mutable.
    immutable = hash_files([runtime / name for name in assemblies] + [runtime / 'MPUlt_puzzles.txt'])
    return dict(origins=origins, external_origins=external, immutable=immutable,
        mutable_initial=hash_files([runtime / 'MPUlt_settings.txt']),
        reflection_contract='verified', directx_loader='native.directx_runtime.find_directx')


def reusable_receipt(record, identity):
    """The cached receipt if it is a valid v2 receipt for exactly this identity payload, else None."""
    try:
        old = json.loads(record.read_text(encoding='utf-8'))
        if identity_v2.receipt_version(old) != 2 or not identity_v2.identity_intact(old):
            return None
    except (OSError, ValueError, AttributeError):
        return None
    return old if old['identity'] == identity else None


def build(directory):
    sources = native_sources()
    manifest = capture_identity(sources, 'ExperimentProgram')
    identity = manifest['build_identity']
    directory = directory / identity[:16]
    directory.mkdir(parents=True, exist_ok=True)
    host = directory / 'Magic600Experiment.exe'
    record = directory / 'build.json'
    old = reusable_receipt(record, manifest['identity']) or {}
    # The legacy compiler is not deterministic: reuse only the exact executable a valid receipt recorded.
    reusable = (old.get('build_identity') == identity and host.is_file()
        and old.get('executable_sha256') == hash_file(host)
        and host.with_suffix('.exe.config').is_file()
        and hash_file(host.with_suffix('.exe.config')) == hash_file(PRODUCT_CONFIG))
    if not reusable:
        compile_program(COMPILER, host, sources, 'ExperimentProgram', directory / 'compile.log')
    backend = [HERE / name for name in BACKEND_SOURCES] + [ROOT / name for name in SHARED_BACKEND_SOURCES]
    product = manifest['identity']['product']
    manifest.update(identity_v2.legacy_aliases(product, ROOT, sources, backend),
        artifacts=identity_v2.hash_inputs([host, host.with_suffix('.exe.config')], ROOT),
        executable_sha256=hash_file(host),
        backend_sha256=product['sources'][identity_v2.relative(HERE / 'adapter.py', ROOT)],
        stage='Post-approval development; final acceptance not yet established',
        approvals=dict(G1='Approved by user 2026-09-16: a156 sample', G2='Approved by user 2026-09-16: a156 sample'))
    unchanged = check_evidence(manifest, 'after_build')
    write_evidence(record, manifest)
    if not unchanged:
        raise RuntimeError('Native sources changed during compilation; this build is not identified or launched. Rebuild after edits finish.')
    return host


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--mode', choices=('g1', 'g2'), required=True)
    parser.add_argument('--session', default='native-review')
    parser.add_argument('--build-only', action='store_true')
    args = parser.parse_args()
    if not args.session.replace('-', '').replace('_', '').isalnum():
        parser.error('Session name must contain letters, numbers, hyphens or underscores')
    if os.name != 'nt':
        parser.error('This sample requires the retained Windows native runtime')
    directory = HERE / 'native-build'
    host = build(directory)
    print('Native build: ' + str(host.parent), flush=True)
    if args.build_only:
        print(host)
        return 0
    data = HERE / 'sessions' / (args.mode + '-' + args.session)
    runtime = data / 'runtime'
    record = host.parent / 'build.json'
    manifest = json.loads(record.read_text(encoding='utf-8'))
    manifest['runtime'] = prepare_runtime(runtime)
    result = None
    try:
        if not check_evidence(manifest, 'before_engine'):
            raise RuntimeError('Native evidence inputs changed before engine start; rebuild.')
        write_evidence(record, manifest)
        with EngineProcess(ROOT, data, data / 'launch.json', data / 'engine.log',
                engine_command=[sys.executable, '-B', str(HERE / 'engine.py')],
                hidden_console=True, timeout=180, progress=lambda message: print(message, flush=True)) as engine:
            if not check_evidence(manifest, 'before_start'):
                raise RuntimeError('Native evidence inputs changed during engine startup; sample not launched.')
            write_evidence(record, manifest)
            print('Starting ' + args.mode.upper() + ' in the retained native program. Session: ' + str(data), flush=True)
            result = subprocess.call([str(host), str(runtime / 'MPUlt.exe'), engine.info['base'], engine.info['token'], args.mode], cwd=runtime)
    finally:
        unchanged = check_evidence(manifest, 'after_run')
        manifest['run_outcome'] = dict(exit_code=result, returned=result is not None,
            immutable_inputs_unchanged=unchanged, final_acceptance='not-assessed')
        write_evidence(record, manifest)
    return result if unchanged else 1


if __name__ == '__main__':
    sys.exit(main())
